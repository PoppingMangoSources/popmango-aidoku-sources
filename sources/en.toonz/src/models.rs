use crate::BASE_URL;
use aidoku::{
	Chapter,
	alloc::{String, Vec, format},
	imports::std::parse_date,
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct ChapterList {
	pub chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterItem {
	chapter_number: String,
	title: Option<String>,
	slug: String,
	date: Option<String>,
}

#[derive(Deserialize)]
pub struct ReaderImage {
	pub filename: String,
}

impl ChapterItem {
	pub fn into_chapter(self, manga_key: &str) -> Chapter {
		// The chapter route takes the slug's number, as written ("2.1", "84.02").
		let key: String = self
			.slug
			.strip_prefix("chapter-")
			.unwrap_or(&self.slug)
			.into();
		// Avoid quoted literals, which the runner does not parse.
		let date_uploaded = self
			.date
			.as_deref()
			.and_then(|date| date.get(..19))
			.and_then(|date| parse_date(date.replace('T', " "), "yyyy-MM-dd HH:mm:ss"));
		Chapter {
			url: Some(format!("{BASE_URL}/comic/{manga_key}/chapter/{key}")),
			key,
			title: self.title.filter(|title| !title.trim().is_empty()),
			chapter_number: self.chapter_number.parse().ok(),
			date_uploaded,
			..Default::default()
		}
	}
}
