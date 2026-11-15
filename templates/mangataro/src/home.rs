use crate::{
	Params,
	helpers::{
		checked, date, excluded_slugs, link_keys, rating, request, series_key, site_headers,
	},
	models::{MangaFeed, PopularChapters, Title, Updates, viewer},
	settings,
};
use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link, Manga,
	MangaWithChapter, Result,
	alloc::{String, Vec, collections::BTreeSet},
	imports::{
		html::Document,
		net::{Request, Response},
		std::send_partial_result,
	},
	prelude::*,
};

#[derive(Clone, Copy)]
pub enum Section {
	Featured,
	Ranking,
	Recent,
	PopularChapters,
	Updates,
	Followed,
	Popular,
	Novels,
	HighScore,
}

impl Section {
	pub fn title(self) -> &'static str {
		match self {
			Self::Featured => "Featured",
			Self::Ranking => "Most Followed New Manga (1 Month)",
			Self::Recent => "Recently Added",
			Self::PopularChapters => "Popular Chapters (This Week)",
			Self::Updates => "Latest Updates",
			Self::Followed => "Most Followed Manga (This Week)",
			Self::Popular => "Popular Manga (This Week)",
			Self::Novels => "Recently Added Novels",
			Self::HighScore => "High Score Manga",
		}
	}
}

pub fn sections(novels: bool) -> Vec<Section> {
	[
		Section::Featured,
		Section::Ranking,
		Section::Recent,
		Section::PopularChapters,
		Section::Updates,
		Section::Followed,
		Section::Popular,
		Section::Novels,
		Section::HighScore,
	]
	.into_iter()
	.filter(|section| novels || !matches!(section, Section::Novels))
	.collect()
}

pub(crate) enum Data {
	Featured(Vec<Manga>),
	Titles(Vec<Title>),
	Chapters(Vec<MangaWithChapter>),
}

pub fn parse_featured(document: &Document, excluded_types: &[String]) -> Vec<Manga> {
	document
		.select(".carousel-slide")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let link = slide.select_first("h2 a")?;
			let url = link.attr("href")?;
			let key = series_key(&url)?.into();
			let title = link.text().filter(|s| !s.is_empty())?;
			let kind = slide
				.select_first("span.uppercase")
				.and_then(|el| el.text());
			if kind
				.as_ref()
				.is_some_and(|kind| excluded_types.contains(kind))
			{
				return None;
			}
			let mut tags = slide
				.select("div.hidden span")
				.into_iter()
				.flatten()
				.filter_map(|el| el.text())
				.collect::<Vec<_>>();
			tags.sort();
			tags.dedup();
			Some(Manga {
				key,
				title,
				cover: slide.select_first("a img").and_then(|el| el.attr("src")),
				description: slide.select_first("p").and_then(|el| el.text()),
				content_rating: rating(&tags),
				tags: Some(tags),
				viewer: kind.as_deref().map(viewer).unwrap_or_default(),
				..Default::default()
			})
		})
		.collect()
}

pub fn parse_recent(document: &Document) -> Vec<Title> {
	document
		.select("#manga-grid > .group")
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let image = card.select_first("img")?;
			Some(Title {
				title: image.attr("alt").filter(|s| !s.trim().is_empty())?,
				cover: image.attr("src")?,
				url: card.select_first("a[href*='/manga/']")?.attr("href")?,
				..Default::default()
			})
		})
		.collect()
}

pub fn parse_novels(document: &Document) -> Vec<MangaWithChapter> {
	document
		.select("#novels-slider-container .group")
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let image = card.select_first("img")?;
			let title = image.attr("alt").filter(|s| !s.trim().is_empty())?;

			let chapter_link = card.select_first("a[href*='/read/']")?;
			let chapter_url = chapter_link.attr("href")?;
			let (key, Some(chapter_key)) = link_keys(&chapter_url)? else {
				return None;
			};
			let title = if settings::label_novels() && !title.ends_with("(Novel)") {
				format!("{title} (Novel)")
			} else {
				title
			};
			Some(MangaWithChapter {
				manga: Manga {
					key,
					title,
					cover: image.attr("src"),
					..Default::default()
				},
				chapter: Chapter {
					key: chapter_key,
					chapter_number: chapter_link
						.text()?
						.trim_start_matches("Ch.")
						.trim()
						.parse()
						.ok(),
					date_uploaded: card
						.select_first("span")
						.and_then(|el| el.text())
						.and_then(|text| date(&text)),
					url: Some(chapter_url),
					..Default::default()
				},
			})
		})
		.collect()
}

fn parse(section: Section, response: Response) -> Result<Data> {
	let response = checked(response)?;
	match section {
		Section::Ranking | Section::Followed => {
			let feed: MangaFeed = response.get_json_owned()?;
			if !feed.success {
				bail!("Could not load followed manga");
			}
			Ok(Data::Titles(feed.manga))
		}
		Section::PopularChapters => {
			let feed: PopularChapters = response.get_json_owned()?;
			if !feed.success {
				bail!("Could not load popular chapters");
			}
			Ok(Data::Titles(feed.chapters))
		}
		Section::Updates => {
			let feed: Updates = response.get_json_owned()?;
			if !feed.success {
				bail!("Could not load latest updates");
			}
			Ok(Data::Titles(feed.data))
		}
		_ => Ok(Data::Titles(response.get_json_owned()?)),
	}
}

pub(crate) fn component(
	section: Section,
	data: Data,
	excluded_types: &[String],
	blocked: &BTreeSet<String>,
) -> HomeComponent {
	let value = match data {
		Data::Featured(entries) => HomeComponentValue::BigScroller {
			entries: entries
				.into_iter()
				.filter(|m| !blocked.contains(&m.key))
				.collect(),
			auto_scroll_interval: Some(5.0),
		},
		Data::Chapters(entries) => HomeComponentValue::MangaChapterList {
			entries: entries
				.into_iter()
				.filter(|m| !blocked.contains(&m.manga.key))
				.collect(),
			page_size: Some(4),
			listing: None,
		},
		Data::Titles(titles) => {
			let mut seen = BTreeSet::new();
			let titles = titles.into_iter().filter(|t| {
				!excluded_types.contains(&t.manga_type)
					&& t.key()
						.is_some_and(|key| !blocked.contains(key) && seen.insert(String::from(key)))
			});
			match section {
				Section::Updates => HomeComponentValue::MangaChapterList {
					entries: titles.filter_map(|t| t.into_update().ok()).collect(),
					page_size: Some(4),
					listing: None,
				},
				Section::Ranking => HomeComponentValue::MangaList {
					entries: titles
						.filter_map(|t| Manga::try_from(t).ok())
						.map(Link::from)
						.collect(),
					page_size: Some(3),
					ranking: true,
					listing: None,
				},
				_ => HomeComponentValue::Scroller {
					entries: titles
						.filter_map(|t| Manga::try_from(t).ok())
						.map(Link::from)
						.collect(),
					listing: None,
				},
			}
		}
	};
	HomeComponent {
		title: Some(section.title().into()),
		value,
		..Default::default()
	}
}

pub fn get_home(params: &Params) -> Result<HomeLayout> {
	let base_url = params.base_url.as_ref();
	let sections = sections(params.novel_home);
	send_partial_result(&HomePartialResult::Layout(HomeLayout {
		components: sections
			.iter()
			.map(|section| HomeComponent {
				title: Some(section.title().into()),
				value: match section {
					Section::Featured => HomeComponentValue::empty_big_scroller(),
					Section::Ranking => HomeComponentValue::empty_manga_list(),
					Section::Updates | Section::Novels => {
						HomeComponentValue::empty_manga_chapter_list()
					}
					_ => HomeComponentValue::empty_scroller(),
				},
				..Default::default()
			})
			.collect(),
	}));
	let requests = sections
		.iter()
		.filter(|section| !matches!(section, Section::Novels | Section::Recent))
		.map(|section| {
			let path = match section {
				Section::Featured => "/home",
				Section::Ranking => "/auth/most-followed-new-manga?period=1m&limit=15",
				Section::PopularChapters => "/auth/popular-chapters?period=week&limit=15",
				Section::Updates => "/wp-json/manga/v1/latest-chapters?page=1",
				Section::Followed => "/auth/most-followed-manga?period=week&limit=15",
				Section::Popular => "/wp-json/manga/v1/popular?period=week&number=15",
				Section::HighScore => "/wp-json/manga/v1/highscore?type=all&number=15",
				_ => unreachable!(),
			};
			if matches!(section, Section::Updates) {
				Ok(
					site_headers(Request::post(format!("{base_url}{path}"))?, base_url)
						.header("Content-Type", "application/json"),
				)
			} else {
				request(base_url, &format!("{base_url}{path}"))
			}
		})
		.collect::<Result<Vec<_>>>()?;
	let mut data = Vec::new();
	let mut novels = None;
	let mut recent = None;
	let mut failure = None;
	for (section, response) in sections
		.iter()
		.filter(|section| !matches!(section, Section::Novels | Section::Recent))
		.zip(Request::send_all(requests))
	{
		let result = response.map_err(Into::into).and_then(|response| {
			if matches!(section, Section::Featured) {
				let document = checked(response)?.get_html()?;
				recent = Some(Data::Titles(parse_recent(&document)));
				if params.novel_home {
					novels = Some(Data::Chapters(
						if settings::excluded_types().iter().any(|t| t == "Novel") {
							Vec::new()
						} else {
							parse_novels(&document)
						},
					));
				}
				Ok(Data::Featured(parse_featured(
					&document,
					&settings::excluded_types(),
				)))
			} else {
				parse(*section, response)
			}
		});
		match result {
			Ok(value) => data.push((*section, value)),
			Err(error) => failure = Some(error),
		}
	}
	if let Some(recent) = recent {
		let index = data
			.iter()
			.position(|(section, _)| !matches!(section, Section::Featured | Section::Ranking))
			.unwrap_or(data.len());
		data.insert(index, (Section::Recent, recent));
	}
	if let Some(novels) = novels {
		let index = data
			.iter()
			.position(|(section, _)| matches!(section, Section::HighScore))
			.unwrap_or(data.len());
		data.insert(index, (Section::Novels, novels));
	}
	if data.is_empty() {
		return Err(failure.unwrap_or_else(|| error!("No home sections")));
	}
	let kinds = data
		.iter()
		.filter_map(|(_, data)| {
			if let Data::Titles(titles) = data {
				Some(titles)
			} else {
				None
			}
		})
		.flatten()
		.filter(|t| !t.manga_type.is_empty())
		.filter_map(|t| Some((String::from(t.key()?), t.manga_type.clone())))
		.collect::<aidoku::alloc::collections::BTreeMap<_, _>>();
	for (_, row) in &mut data {
		if let Data::Titles(titles) = row {
			for title in titles {
				if title.manga_type.is_empty()
					&& let Some(kind) = title.key().and_then(|key| kinds.get(key))
				{
					title.manga_type = kind.clone();
				}
			}
		}
	}
	let keys = data.iter().flat_map(|(_, data)| match data {
		Data::Featured(entries) => entries.iter().map(|m| m.key.as_str()).collect::<Vec<_>>(),
		Data::Titles(entries) => entries.iter().filter_map(Title::key).collect(),
		Data::Chapters(entries) => entries.iter().map(|m| m.manga.key.as_str()).collect(),
	});
	let blocked = excluded_slugs(base_url, keys, &settings::excluded_tags())?;
	let types = settings::excluded_types();
	Ok(HomeLayout {
		components: data
			.into_iter()
			.map(|(section, data)| component(section, data, &types, &blocked))
			.collect(),
	})
}
