use crate::{
	helpers::{date, plain_text, series_key},
	settings,
};
use aidoku::{
	Chapter, Manga, MangaStatus, MangaWithChapter, Page, PageContent, Result, Viewer,
	alloc::{String, Vec, vec},
	prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
pub struct Title {
	#[serde(alias = "manga_title")]
	pub title: String,
	pub cover: String,
	#[serde(default, alias = "manga_url")]
	pub url: String,
	#[serde(default)]
	pub permalink: String,
	#[serde(default)]
	pub manga_permalink: String,
	#[serde(default, alias = "type")]
	pub manga_type: String,
	#[serde(default, alias = "manga_status")]
	pub status: String,
	#[serde(default)]
	pub chapter: String,
	#[serde(default)]
	pub time: String,
}

impl Title {
	pub fn key(&self) -> Option<&str> {
		[&self.manga_permalink, &self.url, &self.permalink]
			.into_iter()
			.find_map(|url| series_key(url))
	}

	pub fn into_update(self) -> Result<MangaWithChapter> {
		let path = self
			.permalink
			.split("/read/")
			.nth(1)
			.ok_or(error!("Missing update chapter"))?;
		let key = String::from(path.trim_end_matches('/'));
		let chapter_number = self.chapter.trim_start_matches("Ch.").trim().parse().ok();
		let chapter = Chapter {
			key,
			chapter_number,
			date_uploaded: date(&self.time),
			url: Some(self.permalink.clone()),
			..Default::default()
		};
		Ok(MangaWithChapter {
			manga: self.try_into()?,
			chapter,
		})
	}
}

impl TryFrom<Title> for Manga {
	type Error = aidoku::AidokuError;

	fn try_from(value: Title) -> Result<Self> {
		let key = value
			.key()
			.filter(|s| !s.is_empty())
			.ok_or(error!("Missing series key"))?
			.into();
		let mut title = plain_text(&value.title)?;
		if title.is_empty() {
			bail!("Missing series title");
		}
		if value.manga_type == "Novel" && settings::label_novels() && !title.ends_with("(Novel)") {
			title.push_str(" (Novel)");
		}
		Ok(Manga {
			key,
			title,
			cover: Some(value.cover),
			status: status(&value.status),
			..Default::default()
		})
	}
}

pub fn status(value: &str) -> MangaStatus {
	match value.to_ascii_lowercase().as_str() {
		"ongoing" => MangaStatus::Ongoing,
		"completed" => MangaStatus::Completed,
		"cancelled" => MangaStatus::Cancelled,
		"hiatus" => MangaStatus::Hiatus,
		_ => MangaStatus::Unknown,
	}
}

pub fn viewer(kind: &str) -> Viewer {
	match kind {
		"Novel" => Viewer::Unknown,
		"Manhwa" | "Manhua" | "Webtoon" => Viewer::Webtoon,
		_ => Viewer::RightToLeft,
	}
}

#[derive(Deserialize)]
pub struct MangaFeed {
	pub success: bool,
	pub manga: Vec<Title>,
}

#[derive(Deserialize)]
pub struct PopularChapters {
	pub success: bool,
	pub chapters: Vec<Title>,
}

#[derive(Deserialize)]
pub struct Updates {
	pub success: bool,
	pub data: Vec<Title>,
}

#[derive(Deserialize)]
pub struct Metadata {
	#[serde(rename = "@type")]
	pub kind: String,
	#[serde(default)]
	pub image: Option<String>,
	#[serde(default)]
	pub author: Option<Credit>,
	#[serde(default)]
	pub genre: String,
	#[serde(default)]
	pub status: String,
}

#[derive(Deserialize)]
pub struct Credit {
	pub name: String,
}

#[derive(Deserialize)]
pub struct ChapterList {
	pub success: bool,
	pub chapters: Vec<ChapterItem>,
	pub has_more: bool,
}

#[derive(Deserialize)]
pub struct ChapterItem {
	pub chapter: String,
	pub title: Option<String>,
	pub date: String,
	pub group_name: Option<String>,
	pub language: String,
	pub url: String,
}

impl TryFrom<ChapterItem> for Chapter {
	type Error = aidoku::AidokuError;

	fn try_from(value: ChapterItem) -> Result<Self> {
		let key = value
			.url
			.split("/read/")
			.nth(1)
			.filter(|s| !s.trim_matches('/').is_empty())
			.ok_or(error!("Missing chapter key"))?
			.trim_end_matches('/')
			.into();
		Ok(Self {
			key,
			title: value
				.title
				.filter(|s| !s.trim().is_empty() && s != "N/A")
				.map(|s| plain_text(&s))
				.transpose()?,
			chapter_number: value.chapter.parse().ok(),
			date_uploaded: date(&value.date),
			scanlators: value
				.group_name
				.filter(|s| !s.trim().is_empty())
				.map(|s| aidoku::alloc::vec![s]),
			url: Some(value.url),
			..Default::default()
		})
	}
}

#[derive(Deserialize)]
pub struct ChapterContent {
	pub success: bool,
	#[serde(default)]
	pub message: Option<String>,
	#[serde(default)]
	pub chapter_type: String,
	#[serde(default)]
	pub images: Vec<String>,
	#[serde(default)]
	pub content: String,
}

#[derive(Serialize)]
pub struct SearchParams {
	pub page: i32,
	pub search: String,
	pub years: String,
	pub genres: String,
	pub types: String,
	pub statuses: String,
	pub sort: String,
	#[serde(rename = "genreMatchMode")]
	pub genre_match_mode: String,
	#[serde(skip)]
	pub excluded_tags: Vec<String>,
	#[serde(skip)]
	pub excluded_types: Vec<String>,
}

#[derive(Deserialize)]
pub struct Slug {
	pub slug: String,
}

impl TryFrom<ChapterContent> for Vec<Page> {
	type Error = aidoku::AidokuError;

	fn try_from(content: ChapterContent) -> Result<Self> {
		if !content.success {
			bail!(
				"{}",
				content
					.message
					.as_deref()
					.unwrap_or("Chapter is unavailable")
			);
		}
		if content.chapter_type == "text" {
			let text = crate::markdown::parse(&content.content)?;
			if text.trim().is_empty() {
				bail!("Chapter has no text");
			}
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}
		if content.images.is_empty() || content.images.iter().any(|url| url.trim().is_empty()) {
			bail!("Chapter has missing pages");
		}
		Ok(content
			.images
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}
