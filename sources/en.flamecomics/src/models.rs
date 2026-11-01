use crate::{BASE_URL, CDN_URL, helpers::paragraphs};
use aidoku::{
	Chapter, ContentRating, Link, LinkValue, Manga, MangaStatus, MangaWithChapter, Viewer,
	alloc::{String, Vec, collections::BTreeMap, string::ToString},
	prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct NextData<T> {
	pub props: Props<T>,
}

#[derive(Deserialize)]
pub struct Props<T> {
	#[serde(rename = "pageProps")]
	pub page_props: T,
}

/// A series or novel as browse, the home blocks and the details pages list it.
#[derive(Deserialize)]
pub struct Entry {
	pub series_id: Option<u32>,
	pub novel_id: Option<u32>,
	pub title: String,
	pub cover: Option<String>,
	pub last_edit: Option<i64>,
	pub status: Option<String>,
	pub r#type: Option<String>,
	pub year: Option<i32>,
	#[serde(rename = "popularityRank")]
	pub popularity_rank: Option<u32>,
	pub description: Option<String>,
	// Browse calls the tags `categories`.
	#[serde(default, alias = "categories")]
	pub tags: Vec<String>,
	#[serde(default)]
	pub author: Vec<String>,
	#[serde(default)]
	pub artist: Vec<String>,
	#[serde(default)]
	pub chapters: Vec<ChapterItem>,
	#[serde(flatten)]
	pub unlock: Unlock,
}

/// How a novel releases its early-access chapters.
#[derive(Default, Deserialize)]
pub struct Unlock {
	pub unlock_policy: Option<String>,
	pub locked_chapter_count: Option<u32>,
	pub locked_chapter_percent: Option<u32>,
	pub locked_chapter_max: Option<u32>,
}

impl Entry {
	/// Novel and series ids overlap, so the key keeps the site's path.
	pub fn key(&self) -> String {
		match self.novel_id {
			Some(id) => format!("/novel/{id}"),
			None => format!("/series/{}", self.series_id.unwrap_or_default()),
		}
	}

	fn cover_url(&self) -> Option<String> {
		let (folder, id) = match self.novel_id {
			Some(id) => ("novels", id),
			None => ("series", self.series_id?),
		};
		let file = self.cover.as_deref()?;
		Some(format!(
			"{CDN_URL}/uploads/images/{folder}/{id}/{file}?{}",
			self.last_edit.unwrap_or_default()
		))
	}

	/// A card: key, title and cover only.
	pub fn into_card(self) -> Manga {
		Manga {
			key: self.key(),
			cover: self.cover_url(),
			title: self.title,
			..Default::default()
		}
	}

	/// A latest row, which names the title's newest chapter.
	pub fn into_latest(mut self) -> Option<MangaWithChapter> {
		let chapter = (!self.chapters.is_empty()).then(|| self.chapters.remove(0))?;
		let url = format!("{BASE_URL}{}", self.key());
		Some(MangaWithChapter {
			chapter: chapter.into_chapter(&url, false),
			manga: self.into_card(),
		})
	}
}

impl From<Entry> for Manga {
	fn from(entry: Entry) -> Self {
		let key = entry.key();
		let status = match entry.status.as_deref().map(str::to_lowercase).as_deref() {
			Some("ongoing") => MangaStatus::Ongoing,
			Some("completed") => MangaStatus::Completed,
			Some("hiatus") => MangaStatus::Hiatus,
			Some("cancelled" | "dropped") => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		};
		let content_rating = if entry
			.tags
			.iter()
			.any(|tag| matches!(tag.as_str(), "Ecchi" | "Ecchi comedy" | "Mature"))
		{
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		};
		let viewer = match entry.r#type.as_deref() {
			_ if entry.novel_id.is_some() => Viewer::Unknown,
			Some("Manga") => Viewer::RightToLeft,
			_ => Viewer::Webtoon,
		};
		let names = |list: Vec<String>| {
			let list: Vec<String> = list
				.into_iter()
				.map(|name| name.trim().to_string())
				.filter(|name| !name.is_empty())
				.collect();
			(!list.is_empty()).then_some(list)
		};
		Manga {
			cover: entry.cover_url(),
			url: Some(format!("{BASE_URL}{key}")),
			description: entry
				.description
				.as_deref()
				.map(|html| paragraphs(html).join("\n\n"))
				.filter(|text| !text.is_empty()),
			authors: names(entry.author),
			artists: names(entry.artist),
			tags: (!entry.tags.is_empty()).then_some(entry.tags),
			key,
			title: entry.title,
			status,
			content_rating,
			viewer,
			..Default::default()
		}
	}
}

#[derive(Deserialize)]
pub struct ChapterItem {
	pub chapter: String,
	pub title: Option<String>,
	pub token: String,
	pub release_date: Option<i64>,
	pub price: Option<String>,
	pub public_release: Option<i64>,
}

impl ChapterItem {
	pub fn number(&self) -> Option<f32> {
		self.chapter.parse().ok()
	}

	pub fn into_chapter(self, manga_url: &str, locked: bool) -> Chapter {
		let key = format!("/{}", self.token);
		Chapter {
			url: Some(format!("{manga_url}{key}")),
			chapter_number: self.number(),
			title: self.title.filter(|title| !title.is_empty()),
			date_uploaded: self.release_date,
			locked,
			key,
			..Default::default()
		}
	}
}

#[derive(Deserialize)]
pub struct HomeProps {
	pub carousel: Vec<Slide>,
	pub novels: Blocks,
	#[serde(rename = "popularEntries")]
	pub popular: Blocks,
	#[serde(rename = "staffPicks")]
	pub staff_picks: Blocks,
	#[serde(rename = "latestEntries")]
	pub latest: Blocks,
	#[serde(rename = "latestNovels")]
	pub latest_novels: Blocks,
}

#[derive(Deserialize)]
pub struct Blocks {
	pub blocks: Vec<Block>,
}

#[derive(Deserialize)]
pub struct Block {
	pub title: String,
	pub series: Vec<Entry>,
}

impl Blocks {
	pub fn into_block(self) -> Option<Block> {
		self.blocks.into_iter().next()
	}
}

#[derive(Deserialize)]
pub struct Slide {
	pub series_id: Option<u32>,
	pub novel_id: Option<u32>,
	pub title: String,
	pub image: String,
	pub link: Option<String>,
}

impl Slide {
	/// A banner opens its series by key; a slide without one opens its link.
	pub fn into_link(self) -> Option<Link> {
		let key = match (self.novel_id, self.series_id) {
			(Some(id), _) => Some(format!("/novel/{id}")),
			(None, Some(id)) => Some(format!("/series/{id}")),
			(None, None) => None,
		};
		let value = match key {
			Some(key) => LinkValue::Manga(Manga {
				key,
				..Default::default()
			}),
			None => LinkValue::Url(self.link.filter(|link| !link.is_empty())?),
		};
		Some(Link {
			title: self.title,
			image_url: Some(format!("{CDN_URL}/uploads/images/carousel/{}", self.image)),
			value: Some(value),
			..Default::default()
		})
	}
}

#[derive(Deserialize)]
pub struct BrowseProps {
	pub series: Vec<Entry>,
}

#[derive(Deserialize)]
pub struct SeriesProps {
	// A novel's page names its entry `novels`.
	#[serde(alias = "novels")]
	pub series: Entry,
	pub chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
pub struct ChapterProps {
	pub chapter: ReaderChapter,
}

#[derive(Deserialize)]
pub struct ReaderChapter {
	pub series_id: Option<u32>,
	pub token: String,
	pub edit_time: Option<i64>,
	#[serde(default)]
	pub images: BTreeMap<u32, Image>,
	pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct Image {
	pub name: String,
}
