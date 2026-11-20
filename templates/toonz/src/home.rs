use crate::{
	Params, Row,
	helpers::{Content, age, card, cards, check, rating, series_path, status},
};
use aidoku::{
	AidokuError, Chapter, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link,
	Listing, ListingKind, Manga, MangaWithChapter, Result,
	alloc::{String, Vec, format},
	imports::{
		html::{Document, Element},
		net::Request,
		std::{current_date, send_partial_result},
	},
	prelude::*,
};

fn section(page: &Document, heading: &str) -> Option<Element> {
	page.select("section")?.find(|section| {
		section
			.select_first("h2")
			.and_then(|title| title.text())
			.is_some_and(|title| title == heading)
	})
}

fn featured(params: &Params, article: &Element) -> Option<(f32, Manga)> {
	let link = article.select_first("h2 a")?;
	let (key, _) = series_path(params, &link.attr("href")?)?;
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
			description: article
				.select_first("h2 + p")
				.and_then(|text| text.text())
				.filter(|text| !text.is_empty()),
			content_rating: rating(cover.has_attr("data-nsfw"), &tags),
			tags: Some(tags).filter(|tags| !tags.is_empty()),
			..Default::default()
		},
	))
}

// Only the first slide's cover is in the page; the others are the same titles' cards further down.
fn slide(params: &Params, heading: &Element, cards: &[Manga]) -> Option<Manga> {
	let link = heading.parent()?;
	let (key, _) = series_path(params, &link.attr("href")?)?;
	let info = link.parent()?;
	let cover = info
		.parent()?
		.select_first("picture img")
		.and_then(|cover| cover.attr("abs:src"))
		.or_else(|| cards.iter().find(|card| card.key == key)?.cover.clone());
	Some(Manga {
		key,
		title: heading.text()?,
		cover,
		description: info
			.select_first("p")
			.and_then(|text| text.text())
			.filter(|text| !text.is_empty()),
		status: info
			.select("[data-slot=badge]")?
			.filter_map(|badge| badge.text())
			.find_map(|text| status(&text))
			.unwrap_or_default(),
		..Default::default()
	})
}

fn chapter(manga: Manga, key: String, number: Option<f32>) -> MangaWithChapter {
	MangaWithChapter {
		chapter: Chapter {
			key,
			chapter_number: number,
			..Default::default()
		},
		manga,
	}
}

fn update(params: &Params, article: &Element) -> Option<MangaWithChapter> {
	let (_, manga) = card(params, &article.select_first("img")?)?;
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
	let parsed = number.parse().ok();
	let mut entry = chapter(manga, number, parsed);
	entry.chapter.date_uploaded = seconds.map(|seconds| current_date() - seconds);
	Some(entry)
}

fn aged_cards(params: &Params, scope: &Element) -> Vec<(i64, Manga)> {
	scope
		.select("img[data-cover]")
		.into_iter()
		.flatten()
		.filter_map(|image| {
			let (link, manga) = card(params, &image)?;
			let seconds = link
				.select("span")?
				.filter_map(|span| span.text())
				.find_map(|text| age(&text))
				.unwrap_or(i64::MAX);
			Some((seconds, manga))
		})
		.collect()
}

fn scroller(entries: Vec<Manga>) -> Option<HomeComponentValue> {
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
}

fn chapter_list(entries: Vec<MangaWithChapter>) -> Option<HomeComponentValue> {
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

pub fn fill(params: &Params, row: Row, pages: &[Option<Document>]) -> Option<HomeComponentValue> {
	let loaded =
		|indices: &'static [usize]| indices.iter().filter_map(|index| pages[*index].as_ref());
	match row {
		Row::Featured(indices) => {
			let mut entries: Vec<_> = loaded(indices)
				.flat_map(|page| {
					page.select("section[aria-label=Featured] article")
						.into_iter()
						.flatten()
				})
				.filter_map(|article| featured(params, &article))
				.collect();
			// Stable, so equal ratings keep the page's order.
			entries.sort_by(|(a, _), (b, _)| b.total_cmp(a));
			(!entries.is_empty()).then(|| HomeComponentValue::BigScroller {
				entries: entries.into_iter().map(|(_, manga)| manga).collect(),
				auto_scroll_interval: Some(5.0),
			})
		}
		Row::Hero(page) => {
			let page = pages[page].as_ref()?;
			let cards = cards(params, page.select("img[data-cover]"));
			let entries: Vec<_> = page
				.select("section h2")?
				.filter_map(|heading| slide(params, &heading, &cards))
				.collect();
			(!entries.is_empty()).then_some(HomeComponentValue::BigScroller {
				entries,
				auto_scroll_interval: Some(5.0),
			})
		}
		Row::Newest(indices, heading) => {
			let mut entries: Vec<_> = loaded(indices)
				.filter_map(|page| section(page, heading))
				.flat_map(|section| aged_cards(params, &section))
				.collect();
			// Stable, so cards of the same age keep the site's order.
			entries.sort_by_key(|(seconds, _)| *seconds);
			scroller(entries.into_iter().map(|(_, manga)| manga).collect())
		}
		Row::Tab(page, heading) => {
			let panel = section(pages[page].as_ref()?, heading)?
				.select_first("div[role=tabpanel]:not([hidden])")?;
			scroller(
				aged_cards(params, &panel)
					.into_iter()
					.map(|(_, manga)| manga)
					.collect(),
			)
		}
		Row::Chapters(page, heading) => {
			let section = section(pages[page].as_ref()?, heading)?;
			let series = cards(params, section.select("img[data-cover]"));
			// "Ch. 47"
			chapter_list(
				section
					.select("a[href*='/chapter/']")?
					.filter_map(|link| {
						let (key, chapter_key) = series_path(params, &link.attr("href")?)?;
						let manga = series.iter().find(|manga| manga.key == key)?.clone();
						let number = link
							.text()
							.as_deref()
							.and_then(|text| text.strip_prefix("Ch.")?.trim().parse().ok());
						Some(chapter(manga, chapter_key?, number))
					})
					.collect(),
			)
		}
		Row::Updates(page) => chapter_list(
			pages[page]
				.as_ref()?
				.select("article")?
				.filter_map(|article| update(params, &article))
				.collect(),
		),
	}
}

pub fn get_home(params: &Params) -> Result<HomeLayout> {
	send_partial_result(&HomePartialResult::Layout(HomeLayout {
		components: params
			.home_rows
			.iter()
			.map(|(title, row)| HomeComponent {
				title: Some(String::from(*title)),
				subtitle: None,
				value: match row {
					Row::Featured(_) | Row::Hero(_) => HomeComponentValue::empty_big_scroller(),
					Row::Chapters(..) | Row::Updates(_) => {
						HomeComponentValue::empty_manga_chapter_list()
					}
					_ => HomeComponentValue::empty_scroller(),
				},
			})
			.collect(),
	}));

	let content = Content::new(params);
	let requests = params
		.home_pages
		.iter()
		.map(|path| content.request(&format!("{}{path}", params.base_url)))
		.collect::<Result<Vec<_>>>()?;
	let mut failure = None;
	let pages: Vec<Option<Document>> = Request::send_all(requests)
		.into_iter()
		.map(|response| {
			response
				.map_err(AidokuError::from)
				.and_then(check)
				.and_then(|response| Ok(response.get_html()?))
				.map_err(|error| failure = Some(error))
				.ok()
		})
		.collect();
	// The final layout removes failed-section placeholders.
	let components: Vec<_> = params
		.home_rows
		.iter()
		.filter_map(|(title, row)| {
			Some(HomeComponent {
				title: Some(String::from(*title)),
				subtitle: None,
				value: fill(params, *row, &pages)?,
			})
		})
		.collect();
	if components.is_empty() {
		return Err(failure.unwrap_or_else(|| error!("No home sections")));
	}
	Ok(HomeLayout { components })
}
