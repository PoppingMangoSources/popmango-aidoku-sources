use crate::{
	api::BASE_URL,
	helpers::{app_language, cover_url},
};
use aidoku::{
	AidokuError, Chapter, ContentRating, Manga, MangaStatus, Result, Viewer,
	alloc::{
		String, Vec,
		collections::{BTreeMap, BTreeSet},
		format, vec,
	},
	helpers::string::PlainText,
	imports::std::parse_date,
	prelude::*,
};
use serde::{Deserialize, Deserializer};

// Novels share the comic catalogue's keys, so theirs are prefixed.
pub const NOVEL_PREFIX: &str = "novel/";

#[derive(Deserialize)]
pub struct Listing<T> {
	pub data: Vec<T>,
	pub total: Option<i64>,
}

// One shape serves listings, the home rows and the comic page.
#[derive(Deserialize)]
pub struct Title {
	title: Option<String>,
	#[serde(default)]
	pub url: String,
	cover: Option<String>,
	#[serde(default)]
	covers: Vec<String>,
	description: Option<String>,
	#[serde(default, deserialize_with = "list")]
	pub genres: Vec<String>,
	#[serde(default, deserialize_with = "list")]
	pub tags: Vec<String>,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	status: Option<String>,
	author: Option<String>,
	#[serde(default)]
	authors: Vec<String>,
	#[serde(default)]
	artists: Vec<String>,
	nsfw: Option<bool>,
	#[serde(default, rename = "availableLangs", alias = "available_langs")]
	pub languages: Vec<String>,
	#[serde(rename = "chapterCount")]
	pub chapter_count: Option<f32>,
}

#[derive(Deserialize)]
pub struct Update {
	title: Option<String>,
	pub url: String,
	cover: Option<String>,
	chapter_id: String,
	number: String,
	pub lang: String,
	released: Option<String>,
	source_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ComicPage {
	pub manga: Title,
	pub chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
pub struct Genres {
	pub genres: Vec<String>,
}

#[derive(Deserialize)]
pub struct ChapterItem {
	pub id: String,
	lang: String,
	pub number: String,
	title: Option<String>,
	pub rank: Option<String>,
	published_date: Option<String>,
	source_name: Option<String>,
	source: Option<String>,
	scan_group: Option<String>,
}

#[derive(Deserialize)]
pub struct Response<T> {
	pub data: T,
}

#[derive(Deserialize)]
pub struct Novel {
	title: String,
	slug: String,
	author: Option<String>,
	description: Option<String>,
	#[serde(default)]
	covers: Vec<String>,
	status: Option<String>,
	#[serde(default)]
	pub chapters: Vec<NovelChapter>,
}

#[derive(Deserialize)]
pub struct NovelChapter {
	number: f32,
	title: Option<String>,
	#[serde(rename = "createdAt")]
	created_at: Option<String>,
}

#[derive(Deserialize)]
pub struct NovelText {
	pub content: String,
}

#[derive(Deserialize)]
pub struct Pages {
	pub chapter: PageUrls,
}

#[derive(Deserialize)]
pub struct PageUrls {
	pub data: Vec<String>,
}

// Some records hold their lists as Python reprs ("['Action', \"Boys' Love\"]").
fn list<'de, D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Vec<String>, D::Error> {
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum List {
		Many(Vec<String>),
		One(String),
	}
	Ok(match Option::<List>::deserialize(deserializer)? {
		Some(List::Many(values)) => values,
		Some(List::One(text)) => {
			let mut values = Vec::new();
			let mut chars = text.chars();
			while let Some(c) = chars.next() {
				if c == '\'' || c == '"' {
					values.push(chars.by_ref().take_while(|next| *next != c).collect());
				}
			}
			values
		}
		None => Vec::new(),
	})
}

fn status(status: Option<&str>) -> MangaStatus {
	match status.map(str::to_lowercase).as_deref() {
		Some("ongoing") => MangaStatus::Ongoing,
		Some("completed") => MangaStatus::Completed,
		Some("hiatus") => MangaStatus::Hiatus,
		Some("cancelled" | "discontinued") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

// "0001-01-01" marks an unknown date; avoid quoted literals, which the runner does not parse.
fn date(value: Option<&str>) -> Option<i64> {
	let value = value.filter(|value| !value.starts_with("0001"))?;
	let day = value.get(..10)?;
	let time = value.get(11..19).unwrap_or("00:00:00");
	parse_date(format!("{day} {time}"), "yyyy-MM-dd HH:mm:ss")
}

impl Title {
	pub fn content_rating(&self) -> ContentRating {
		let has = |names: &[&str]| {
			self.genres
				.iter()
				.chain(&self.tags)
				.any(|tag| names.contains(&tag.as_str()))
		};
		if self.nsfw == Some(true) || has(&["Adult", "Hentai", "Pornographic", "Smut"]) {
			ContentRating::NSFW
		} else if has(&["Ecchi", "Mature"]) {
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		}
	}

	fn into_manga(self, detailed: bool) -> Result<Manga> {
		let content_rating = self.content_rating();
		let title = self
			.title
			.filter(|name| !name.trim().is_empty())
			.ok_or_else(|| error!("Series has no title"))?;
		if self.url.is_empty() {
			bail!("Series has no key");
		}
		let mut manga = Manga {
			cover: self
				.covers
				.into_iter()
				.chain(self.cover)
				.find_map(|cover| cover_url(&cover)),
			content_rating,
			..Default::default()
		};
		if detailed {
			manga.viewer = match self.kind.as_deref().map(str::to_lowercase).as_deref() {
				Some("manhwa" | "manhua") => Viewer::Webtoon,
				_ if self.tags.iter().any(|tag| tag == "Long Strip") => Viewer::Webtoon,
				Some("manga") => Viewer::RightToLeft,
				_ => Viewer::Unknown,
			};
			let mut tags = self.genres;
			tags.extend(self.tags);
			tags.sort_unstable();
			tags.dedup();
			let mut authors = self.authors;
			if authors.is_empty() {
				authors.extend(self.author.filter(|name| !name.trim().is_empty()));
			}
			manga.url = Some(format!("{BASE_URL}/comic/{}", self.url));
			manga.description = self
				.description
				.filter(|text| !text.trim().is_empty())
				.map(|text| text.escape_markdown());
			manga.authors = Some(authors).filter(|authors| !authors.is_empty());
			manga.artists = Some(self.artists).filter(|artists| !artists.is_empty());
			manga.status = status(self.status.as_deref());
			manga.tags = Some(tags);
		}
		manga.key = self.url;
		manga.title = title;
		Ok(manga)
	}

	pub fn into_details(self) -> Result<Manga> {
		self.into_manga(true)
	}
}

impl TryFrom<Title> for Manga {
	type Error = AidokuError;

	fn try_from(title: Title) -> Result<Self> {
		title.into_manga(false)
	}
}

impl Update {
	pub fn uploader(&self) -> &str {
		self.source_name.as_deref().unwrap_or("swarmws")
	}

	pub fn into_manga(self) -> Option<Manga> {
		Some(Manga {
			key: self.url,
			title: self.title.filter(|title| !title.trim().is_empty())?,
			cover: self.cover.as_deref().and_then(cover_url),
			..Default::default()
		})
	}

	pub fn into_chapter(mut self) -> Option<(Manga, Chapter)> {
		let chapter = Chapter {
			url: Some(format!(
				"{BASE_URL}/comic/{}/chapter/{}",
				self.url, self.number
			)),
			key: core::mem::take(&mut self.chapter_id),
			chapter_number: self.number.parse().ok(),
			date_uploaded: date(self.released.as_deref()),
			scanlators: Some(vec![self.uploader().into()]),
			language: Some(app_language(&self.lang).into()),
			..Default::default()
		};
		Some((self.into_manga()?, chapter))
	}
}

impl ChapterItem {
	// The site's Uploader column; a numbered one ("official2") is another copy from that origin.
	fn uploader(&self) -> &str {
		[&self.source_name, &self.source, &self.scan_group]
			.into_iter()
			.flatten()
			.map(|name| name.as_str())
			.find(|name| !name.trim().is_empty())
			.unwrap_or("swarmws")
	}

	fn group(&self) -> String {
		self.uploader()
			.trim_end_matches(|c: char| c.is_ascii_digit())
			.to_lowercase()
	}

	// The site's pick among uploads: the lowest rank letter, then the first copy.
	fn site_order(&self) -> (&str, u32) {
		let uploader = self.uploader();
		let copy = uploader
			.strip_prefix(uploader.trim_end_matches(|c: char| c.is_ascii_digit()))
			.and_then(|digits| digits.parse().ok())
			.unwrap_or(1);
		(self.rank.as_deref().unwrap_or("~"), copy)
	}

	fn into_chapter(self, slug: &str) -> Chapter {
		let number = self.number.parse::<f32>().ok();
		let scanlators = Some(vec![self.uploader().into()]);
		// The reader picks an upload by its rank letter, "a" being the default.
		let query = self
			.rank
			.as_deref()
			.filter(|rank| *rank != "a")
			.map(|rank| format!("?{rank}"))
			.unwrap_or_default();
		Chapter {
			url: Some(format!(
				"{BASE_URL}/comic/{slug}/chapter/{}{query}",
				self.number
			)),
			title: self
				.title
				.filter(|title| *title != format!("Chapter {}", self.number)),
			chapter_number: number,
			date_uploaded: date(self.published_date.as_deref()),
			scanlators,
			language: Some(app_language(&self.lang).into()),
			key: self.id,
			..Default::default()
		}
	}
}

pub fn chapter_list(
	items: Vec<ChapterItem>,
	slug: &str,
	languages: &[String],
	blocked: &[String],
	mode: &str,
	preferred: &[String],
) -> Vec<Chapter> {
	let listed = |names: &[String], item: &ChapterItem| {
		names
			.iter()
			.position(|name| *name == item.uploader().to_lowercase() || *name == item.group())
	};
	let items: Vec<ChapterItem> = items
		.into_iter()
		.filter(|item| {
			languages
				.iter()
				.any(|language| language == app_language(&item.lang))
		})
		.filter(|item| listed(blocked, item).is_none())
		.collect();
	let preference = |item: &ChapterItem| {
		if mode == "chapter" {
			listed(preferred, item).unwrap_or(preferred.len())
		} else {
			0
		}
	};
	// The upload each chapter keeps, by language, number and, per uploader, its group.
	let mut picks: BTreeMap<(&str, &str, String), usize> = BTreeMap::new();
	if mode != "off" {
		for (index, item) in items.iter().enumerate() {
			let group = if mode == "uploader" {
				item.group()
			} else {
				String::new()
			};
			let key = (item.lang.as_str(), item.number.as_str(), group);
			if picks.get(&key).is_none_or(|best| {
				let other = &items[*best];
				(preference(item), item.site_order()) < (preference(other), other.site_order())
			}) {
				picks.insert(key, index);
			}
		}
	}
	let kept: BTreeSet<usize> = picks.into_values().collect();
	items
		.into_iter()
		.enumerate()
		.filter(|(index, _)| mode == "off" || kept.contains(index))
		.map(|(_, item)| item.into_chapter(slug))
		.collect()
}

impl Novel {
	pub fn into_manga(self) -> Manga {
		Manga {
			key: format!("{NOVEL_PREFIX}{}", self.slug),
			title: self.title,
			cover: self.covers.first().and_then(|cover| cover_url(cover)),
			..Default::default()
		}
	}

	pub fn into_details(mut self) -> Manga {
		let url = format!("{BASE_URL}/novel/{}", self.slug);
		let chapters = core::mem::take(&mut self.chapters)
			.into_iter()
			.map(|chapter| {
				let number = format!("{}", chapter.number);
				Chapter {
					url: Some(format!("{url}/chapter/{number}")),
					title: chapter.title,
					chapter_number: Some(chapter.number),
					date_uploaded: date(chapter.created_at.as_deref()),
					language: Some("en".into()),
					key: number,
					..Default::default()
				}
			})
			.collect();
		Manga {
			url: Some(url),
			description: self
				.description
				.take()
				.filter(|text| !text.trim().is_empty())
				.map(|text| text.escape_markdown()),
			authors: self.author.take().map(|author| vec![author]),
			status: status(self.status.as_deref()),
			viewer: Viewer::Vertical,
			chapters: Some(chapters),
			..self.into_manga()
		}
	}
}
