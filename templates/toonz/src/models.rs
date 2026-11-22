use crate::{Params, helpers::chapter_url};
use aidoku::{
	Chapter,
	alloc::{String, Vec},
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
pub struct Genre {
	pub name: String,
	pub slug: String,
}

#[derive(Deserialize)]
pub struct ReaderImage {
	pub filename: String,
}

impl ChapterItem {
	// The chapter route takes the slug after "chapter-", as written ("2.1", "45-wad21").
	pub fn key(&self) -> &str {
		self.slug.strip_prefix("chapter-").unwrap_or(&self.slug)
	}

	pub fn number(&self) -> Option<f32> {
		self.chapter_number.parse().ok()
	}

	pub fn into_chapter(self, params: &Params, manga_key: &str) -> Chapter {
		let key = String::from(self.key());
		let chapter_number = self.number();
		// Avoid quoted literals, which the runner does not parse.
		let date_uploaded = self
			.date
			.as_deref()
			.and_then(|date| date.get(..19))
			.and_then(|date| parse_date(date.replace('T', " "), "yyyy-MM-dd HH:mm:ss"));
		Chapter {
			url: Some(chapter_url(params, manga_key, &key)),
			key,
			title: self.title.filter(|title| !title.trim().is_empty()),
			chapter_number,
			date_uploaded,
			..Default::default()
		}
	}
}
