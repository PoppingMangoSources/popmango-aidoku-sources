use crate::helpers::{chapter_url, manga_url, title_type};
use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Viewer,
	alloc::{String, Vec, format, string::ToString, vec},
	imports::std::parse_date,
};
use serde::Deserialize;

const CDN_URL: &str = "https://bulbasaur.poke-black-and-white.net/covers";

#[derive(Deserialize)]
pub struct Data<T> {
	pub data: T,
}

#[derive(Deserialize)]
pub struct Paged<T> {
	pub data: Vec<T>,
	pub pagination: Pagination,
}

#[derive(Deserialize)]
pub struct Pagination {
	pub page: i32,
	pub total_pages: i32,
}

#[derive(Deserialize)]
pub struct Title {
	pub id: String,
	name: String,
	image: Option<Image>,
	#[serde(default)]
	description: Vec<String>,
	#[serde(default, rename = "originalLanguage")]
	pub original_language: String,
	status: Option<String>,
	#[serde(default, rename = "availableTranslatedLanguages")]
	pub translated_languages: Vec<String>,
	#[serde(default)]
	pub tags: Vec<Tag>,
	#[serde(default)]
	author: Vec<Author>,
	#[serde(default)]
	pub is18plus: bool,
	#[serde(default)]
	pub recent_chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
struct Image {
	file: Option<String>,
	cdn_mangadex: Option<String>,
	cdn_mangaupdate: Option<String>,
	cdn_mangaupdates: Option<String>,
	cover: Option<Cover>,
}

#[derive(Deserialize)]
struct Cover {
	path: Option<String>,
}

#[derive(Deserialize)]
pub struct Tag {
	pub id: String,
	name: String,
}

#[derive(Deserialize)]
struct Author {
	name: String,
}

#[derive(Deserialize)]
pub struct ChapterItem {
	id: String,
	name: Option<String>,
	number: Option<f32>,
	#[serde(default)]
	volume: f32,
	lang: String,
	pub group_name: Option<String>,
	created_at: Option<String>,
}

#[derive(Deserialize)]
pub struct ChapterDetail {
	pub chapter: ChapterPages,
}

#[derive(Deserialize)]
pub struct ChapterPages {
	pub title_id: Option<String>,
	#[serde(default)]
	pub pages: Vec<String>,
}

impl Image {
	// The site prefers its own copy over the external mirrors.
	fn into_url(self) -> Option<String> {
		let hosted = self
			.cover
			.and_then(|cover| cover.path)
			.filter(|path| !path.is_empty())
			.map(|path| format!("{CDN_URL}/{}", path.replace('\\', "/")));
		[
			hosted,
			self.file,
			self.cdn_mangadex,
			self.cdn_mangaupdate,
			self.cdn_mangaupdates,
		]
		.into_iter()
		.flatten()
		.find(|url| !url.is_empty())
	}
}

impl Title {
	fn into_manga(self, detailed: bool) -> Manga {
		let has_tag = |names: &[&str]| {
			self.tags
				.iter()
				.any(|tag| names.contains(&tag.name.as_str()))
		};
		// The site only flags 18+ titles; the rest are rated from their tags.
		let content_rating = if self.is18plus || has_tag(&["Adult", "Hentai", "Manhwa 18+", "Smut"])
		{
			ContentRating::NSFW
		} else if has_tag(&["Ecchi", "Mature"]) {
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		};
		let viewer = match title_type(&self.original_language) {
			Some("manhwa" | "manhua") => Viewer::Webtoon,
			_ if has_tag(&["Long Strip"]) => Viewer::Webtoon,
			Some("manga") => Viewer::RightToLeft,
			_ => Viewer::Unknown,
		};
		let status = match self.status.as_deref().map(str::to_lowercase).as_deref() {
			Some("ongoing") => MangaStatus::Ongoing,
			Some("completed") => MangaStatus::Completed,
			Some("hiatus" | "on_hold") => MangaStatus::Hiatus,
			Some("cancelled") => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		};
		let mut manga = Manga {
			cover: self.image.and_then(Image::into_url),
			status,
			content_rating,
			..Default::default()
		};
		if detailed {
			let mut tags: Vec<_> = self.tags.into_iter().map(|tag| tag.name).collect();
			tags.sort_unstable();
			manga.url = Some(manga_url(&self.id));
			manga.description = Some(self.description.join("\n\n")).filter(|text| !text.is_empty());
			manga.authors = Some(self.author.into_iter().map(|author| author.name).collect());
			manga.tags = Some(tags);
			manga.viewer = viewer;
		}
		manga.key = self.id;
		manga.title = self.name;
		manga
	}

	pub fn into_details(self) -> Manga {
		self.into_manga(true)
	}
}

impl From<Title> for Manga {
	fn from(title: Title) -> Self {
		title.into_manga(false)
	}
}

impl ChapterItem {
	// A chapter in a language the reader hasn't selected is `None`.
	pub fn into_chapter(self, languages: &[(String, String)]) -> Option<Chapter> {
		let lang = self.lang.to_lowercase();
		let language = languages.iter().find(|(code, _)| *code == lang)?.1.clone();
		// Most names only repeat the number, such as "Ch.22".
		let title = self.name.filter(|name| {
			!name.is_empty()
				&& !self
					.number
					.is_some_and(|number| name.contains(&number.to_string()))
		});
		// Timestamps are UTC with microseconds; the format avoids a quoted `T`.
		let date_uploaded = self
			.created_at
			.as_deref()
			.and_then(|date| date.get(..19))
			.and_then(|date| parse_date(date.replace('T', " "), "yyyy-MM-dd HH:mm:ss"));
		Some(Chapter {
			url: Some(chapter_url(&self.id)),
			key: self.id,
			title,
			chapter_number: self.number,
			volume_number: (self.volume > 0.0).then_some(self.volume),
			date_uploaded,
			scanlators: self.group_name.map(|group| vec![group]),
			language: Some(language),
			..Default::default()
		})
	}
}
