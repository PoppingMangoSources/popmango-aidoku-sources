use crate::{
	BASE_URL, Toonz,
	helpers::{Content, age, card, rating, series_path},
};
use aidoku::{
	AidokuError, Chapter, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult,
	Link, Listing, ListingKind, Manga, MangaWithChapter, Result,
	alloc::{String, Vec, format},
	imports::{
		html::{Document, Element},
		net::Request,
		std::{current_date, send_partial_result},
	},
	prelude::*,
};

const MANHWA: usize = 0;
const MANGA: usize = 1;

#[derive(Clone, Copy)]
pub enum Feed {
	TopRated,
	NewTitles,
	Latest,
	Tab(usize, &'static str),
}

pub const SECTIONS: [(&str, Feed); 7] = [
	("Top Rated This Month", Feed::TopRated),
	("New Titles", Feed::NewTitles),
	("Recently Updated", Feed::Latest),
	(
		"Popular Updates (Manga)",
		Feed::Tab(MANGA, "Popular Updates"),
	),
	(
		"Popular Updates (Manhwa)",
		Feed::Tab(MANHWA, "Popular Updates"),
	),
	("Trending Manga", Feed::Tab(MANGA, "Trending")),
	("Trending Manhwa", Feed::Tab(MANHWA, "Trending")),
];

// The manhwa and manga home pages, then the all-type list, which shows update times.
const PAGES: [&str; 3] = ["/manhwa", "/manga", "/comics?sort=latest&view=list"];

fn section(page: &Document, heading: &str) -> Option<Element> {
	page.select("section")?.find(|section| {
		section
			.select_first("h2")
			.and_then(|title| title.text())
			.is_some_and(|title| title == heading)
	})
}

fn featured(article: &Element) -> Option<(f32, Manga)> {
	let link = article.select_first("h2 a")?;
	let (key, _) = series_path(&link.attr("href")?)?;
	let cover = article.select_first("img[data-cover]")?;
	let mut tags: Vec<String> = article
		.select("a[href^='/genre/']")?
		.filter_map(|tag| tag.text())
		.collect();
	tags.sort_unstable();
	tags.dedup();
	// "5.0 rating"
	let score = article
		.select("dl > div")?
		.find(|row| {
			row.select_first("dt")
				.and_then(|label| label.text())
				.as_deref() == Some("Rating")
		})
		.and_then(|row| row.select_first("dd")?.text())
		.and_then(|text| text.split(' ').next()?.parse().ok())
		.unwrap_or_default();
	Some((
		score,
		Manga {
			key,
			title: link.text()?,
			cover: cover.attr("abs:src"),
			description: article.select_first("h2 + p").and_then(|text| text.text()),
			content_rating: rating(cover.has_attr("data-nsfw"), &tags),
			tags: Some(tags),
			..Default::default()
		},
	))
}

fn latest(article: &Element) -> Option<MangaWithChapter> {
	let manga = card(&article.select_first("a[href]")?)?;
	let number = article
		.select("span")?
		.filter_map(|span| span.text())
		.find_map(|text| Some(String::from(text.strip_prefix("CH.")?.trim())))?;
	let seconds = article
		.select("p")?
		.next_back()?
		.text()
		.as_deref()
		.and_then(age);
	Some(MangaWithChapter {
		chapter: Chapter {
			url: Some(format!("{BASE_URL}/comic/{}/chapter/{number}", manga.key)),
			chapter_number: number.parse().ok(),
			date_uploaded: seconds.map(|seconds| current_date() - seconds),
			key: number,
			..Default::default()
		},
		manga,
	})
}

// Cards with how long ago each was added.
fn aged_cards(scope: &Element) -> Vec<(i64, Manga)> {
	scope
		.select("a[href]")
		.into_iter()
		.flatten()
		.filter_map(|link| {
			let seconds = link
				.select("span")?
				.filter_map(|span| span.text())
				.find_map(|text| age(&text))
				.unwrap_or(i64::MAX);
			Some((seconds, card(&link)?))
		})
		.collect()
}

pub fn fill(feed: Feed, pages: &[Option<Document>]) -> Option<HomeComponentValue> {
	let types = || pages[..2].iter().flatten();
	let row = |entries: Vec<Manga>| {
		let mut keys: Vec<String> = Vec::new();
		let entries: Vec<Link> = entries
			.into_iter()
			.filter(|manga| {
				let new = !keys.contains(&manga.key);
				if new {
					keys.push(manga.key.clone());
				}
				new
			})
			.map(Link::from)
			.collect();
		(!entries.is_empty()).then_some(HomeComponentValue::Scroller {
			entries,
			listing: None,
		})
	};
	match feed {
		Feed::TopRated => {
			let mut entries: Vec<_> = types()
				.flat_map(|page| {
					page.select("section[aria-label=Featured] article")
						.into_iter()
						.flatten()
				})
				.filter_map(|article| featured(&article))
				.collect();
			entries.sort_by(|(a, _), (b, _)| b.total_cmp(a));
			(!entries.is_empty()).then(|| HomeComponentValue::BigScroller {
				entries: entries.into_iter().map(|(_, manga)| manga).collect(),
				auto_scroll_interval: Some(5.0),
			})
		}
		Feed::NewTitles => {
			let mut entries: Vec<_> = types()
				.filter_map(|page| section(page, "New Titles"))
				.flat_map(|section| aged_cards(&section))
				.collect();
			entries.sort_by_key(|(seconds, _)| *seconds);
			row(entries.into_iter().map(|(_, manga)| manga).collect())
		}
		Feed::Latest => {
			let entries: Vec<_> = pages[2]
				.as_ref()?
				.select("article")?
				.filter_map(|article| latest(&article))
				.collect();
			(!entries.is_empty()).then(|| HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries,
				listing: Some(Listing {
					id: "latest".into(),
					name: "Latest Updates".into(),
					kind: ListingKind::Default,
				}),
			})
		}
		Feed::Tab(page, heading) => {
			let panel = section(pages[page].as_ref()?, heading)?
				.select_first("div[role=tabpanel]:not([hidden])")?;
			row(aged_cards(&panel)
				.into_iter()
				.map(|(_, manga)| manga)
				.collect())
		}
	}
}

impl Home for Toonz {
	fn get_home(&self) -> Result<HomeLayout> {
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: SECTIONS
				.iter()
				.map(|(title, feed)| HomeComponent {
					title: Some(String::from(*title)),
					subtitle: None,
					value: match feed {
						Feed::TopRated => HomeComponentValue::empty_big_scroller(),
						Feed::Latest => HomeComponentValue::empty_manga_chapter_list(),
						_ => HomeComponentValue::empty_scroller(),
					},
				})
				.collect(),
		}));

		let content = Content::new();
		let requests = PAGES
			.iter()
			.map(|path| content.request(&format!("{BASE_URL}{path}")))
			.collect::<Result<Vec<_>>>()?;
		let mut failure = None;
		let pages: Vec<Option<Document>> = Request::send_all(requests)
			.into_iter()
			.map(|response| {
				response
					.map_err(AidokuError::from)
					.and_then(|response| Ok(response.get_html()?))
					.map_err(|error| failure = Some(error))
					.ok()
			})
			.collect();
		// The final layout removes failed-section placeholders.
		let components: Vec<_> = SECTIONS
			.iter()
			.filter_map(|(title, feed)| {
				Some(HomeComponent {
					title: Some(String::from(*title)),
					subtitle: None,
					value: fill(*feed, &pages)?,
				})
			})
			.collect();
		if components.is_empty() {
			return Err(failure.unwrap_or_else(|| error!("No home sections")));
		}
		Ok(HomeLayout { components })
	}
}
