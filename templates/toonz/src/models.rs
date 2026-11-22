use crate::{Params, helpers::chapter_url};
use aidoku::{
	Chapter,
	alloc::{String, Vec},
	imports::std::parse_date_with_options,
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
	pub fn key(&self) -> &str {
		self.slug.strip_prefix("chapter-").unwrap_or(&self.slug)
	}

	pub fn number(&self) -> Option<f32> {
		self.chapter_number.parse().ok()
	}

	pub fn into_chapter(self, params: &Params, manga_key: &str) -> Chapter {
		let key = String::from(self.key());
		let chapter_number = self.number();
		// The API uses UTC. Removing the ISO separator also works in the test runner.
		let date_uploaded = self
			.date
			.as_deref()
			.and_then(|date| date.get(..19))
			.and_then(|date| {
				parse_date_with_options(
					date.replace('T', " "),
					"yyyy-MM-dd HH:mm:ss",
					"en_US_POSIX",
					"UTC",
				)
			});
		Chapter {
			url: Some(chapter_url(params, manga_key, &key)),
			key,
			title: self.title.filter(|title| {
				let title = title.trim();
				let label = title
					.strip_prefix("Chapter ")
					.or_else(|| title.strip_prefix("Ch."))
					.unwrap_or(title);
				!title.is_empty()
					&& label
						.trim()
						.parse::<f32>()
						.ok()
						.is_none_or(|number| Some(number) != chapter_number)
			}),
			chapter_number,
			date_uploaded,
			..Default::default()
		}
	}
}
