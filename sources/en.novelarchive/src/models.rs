use crate::{
	BASE_URL,
	helpers::{chapter_title, content_rating, genres},
};
use aidoku::{
	Chapter, Manga, MangaStatus, MangaWithChapter,
	alloc::{String, Vec, string::ToString, vec},
	helpers::string::PlainText,
	imports::std::parse_date,
	prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ErrorResponse {
	pub error: String,
}

/// A novel as the lists and the details endpoint return it; only details fill `chapter_names`.
#[derive(Deserialize)]
pub struct Novel {
	pub id: String,
	#[serde(default)]
	title: String,
	author: Option<String>,
	#[serde(default)]
	genres: String,
	cover_url: Option<String>,
	novel_image: Option<String>,
	image_url: Option<String>,
	description: Option<String>,
	release_status: Option<String>,
	total_chapters: Option<String>,
	#[serde(default)]
	chapter_names: Vec<String>,
	updated_at: Option<String>,
}

#[derive(Deserialize)]
pub struct NovelList {
	pub novels: Vec<Novel>,
	pub pagination: Option<Pagination>,
}

#[derive(Deserialize)]
pub struct Pagination {
	pub has_next: bool,
}

#[derive(Deserialize)]
pub struct NovelDetails {
	pub novel: Novel,
}

#[derive(Deserialize)]
pub struct ChapterResponse {
	pub chapter: ChapterText,
}

#[derive(Deserialize)]
pub struct ChapterText {
	pub content: Option<String>,
}

#[derive(Deserialize)]
pub struct GenreList {
	pub genres: Vec<Genre>,
}

#[derive(Deserialize)]
pub struct Genre {
	pub value: String,
	pub label: String,
}

impl Novel {
	// The site falls back through the three image fields in this order.
	fn cover(&self) -> Option<String> {
		let path = [&self.cover_url, &self.novel_image, &self.image_url]
			.into_iter()
			.flatten()
			.map(|path| path.trim())
			.find(|path| !path.is_empty())?;
		Some(if path.starts_with("http") {
			path.into()
		} else {
			format!("{BASE_URL}{path}")
		})
	}

	fn chapter_count(&self) -> usize {
		self.total_chapters
			.as_deref()
			.and_then(|count| count.trim().parse().ok())
			.unwrap_or_default()
	}

	pub fn into_details(self) -> Manga {
		let status = match self
			.release_status
			.as_deref()
			.map(str::to_lowercase)
			.as_deref()
		{
			Some("ongoing") => MangaStatus::Ongoing,
			Some("completed") => MangaStatus::Completed,
			Some("hiatus") => MangaStatus::Hiatus,
			Some("cancelled" | "dropped") => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		};
		let tags = genres(&self.genres);
		Manga {
			cover: self.cover(),
			url: Some(format!("{BASE_URL}/novel?id={}", self.id)),
			title: self.title.trim().into(),
			authors: self
				.author
				.map(|author| author.trim().to_string())
				.filter(|author| !author.is_empty() && author != "Unknown")
				.map(|author| vec![author]),
			description: self
				.description
				.map(|description| description.trim().escape_markdown())
				.filter(|description| !description.is_empty()),
			content_rating: content_rating(&self.genres),
			tags: (!tags.is_empty()).then_some(tags),
			status,
			key: self.id,
			..Default::default()
		}
	}

	/// Chapter n is the n-th name; a novel whose names were never collected still has a count.
	pub fn chapters(&self) -> Vec<Chapter> {
		let count = self.chapter_names.len().max(self.chapter_count());
		(1..=count)
			.rev()
			.map(|number| Chapter {
				key: number.to_string(),
				title: self
					.chapter_names
					.get(number - 1)
					.and_then(|name| chapter_title(name, number)),
				chapter_number: Some(number as f32),
				url: Some(format!(
					"{BASE_URL}/reader?novel={}&chapter={number}",
					self.id
				)),
				..Default::default()
			})
			.collect()
	}

	/// The feed names no chapter, so the newest is the last one the novel counts.
	pub fn into_latest(self) -> Option<MangaWithChapter> {
		let number = self.chapter_count();
		if number == 0 {
			return None;
		}
		let date_uploaded = self
			.updated_at
			.as_deref()
			.and_then(|date| parse_date(date, "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'"));
		let url = format!("{BASE_URL}/reader?novel={}&chapter={number}", self.id);
		Some(MangaWithChapter {
			manga: self.into(),
			chapter: Chapter {
				key: number.to_string(),
				chapter_number: Some(number as f32),
				date_uploaded,
				url: Some(url),
				..Default::default()
			},
		})
	}
}

/// A card: key, title, cover and content rating only.
impl From<Novel> for Manga {
	fn from(novel: Novel) -> Self {
		Manga {
			cover: novel.cover(),
			title: novel.title.trim().into(),
			content_rating: content_rating(&novel.genres),
			key: novel.id,
			..Default::default()
		}
	}
}
