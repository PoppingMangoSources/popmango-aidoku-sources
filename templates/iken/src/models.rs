use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Result, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{element::ElementHelpers, string::PlainText},
	imports::{html::Html, std::parse_date_with_options},
	prelude::*,
};
use serde::{Deserialize, Deserializer};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResponse {
	pub posts: Vec<Post>,
	pub total_count: i32,
}
#[derive(Deserialize)]
pub struct PostResponse {
	pub post: Post,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChaptersResponse {
	pub post: ChapterList,
	pub total_chapter_count: usize,
}
#[derive(Deserialize)]
pub struct ChapterList {
	pub chapters: Vec<IkenChapter>,
}
#[derive(Deserialize)]
pub struct ChapterResponse {
	pub chapter: IkenChapter,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Post {
	pub id: i32,
	pub slug: String,
	post_title: String,
	post_content: Option<String>,
	featured_image: Option<String>,
	author: Option<String>,
	artist: Option<String>,
	series_type: Option<String>,
	series_status: Option<String>,
	genres: Option<Vec<Genre>>,
	#[serde(rename = "is18Plus")]
	is_18_plus: Option<bool>,
}
#[derive(Deserialize)]
pub struct Genre {
	name: String,
}
#[derive(Deserialize)]
pub struct Author {
	name: String,
}
#[derive(Deserialize)]
pub struct ChapterPost {
	pub slug: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IkenChapter {
	pub id: i32,
	pub slug: String,
	#[serde(deserialize_with = "number")]
	number: f32,
	title: Option<String>,
	created_by: Option<Author>,
	team: Option<Author>,
	created_at: Option<String>,
	is_locked: Option<bool>,
	is_time_locked: Option<bool>,
	is_permanently_locked: Option<bool>,
	is_locked_by_coins: Option<bool>,
	is_short_link_locked: Option<bool>,
	is_accessible: Option<bool>,
	is_purchased: Option<bool>,
	price: Option<f32>,
	manga_post: Option<ChapterPost>,
	pub content: Option<String>,
	pub images: Option<Vec<Image>>,
}
#[derive(Deserialize)]
pub struct Image {
	pub url: String,
	pub order: Option<i32>,
}

fn number<'de, D: Deserializer<'de>>(deserializer: D) -> core::result::Result<f32, D::Error> {
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum Number {
		Float(f32),
		Text(String),
	}
	match Number::deserialize(deserializer)? {
		Number::Float(value) => Ok(value),
		Number::Text(value) => value.parse().map_err(serde::de::Error::custom),
	}
}

impl Post {
	pub fn key(&self) -> Result<String> {
		if self.id <= 0 || self.slug.trim().is_empty() || self.post_title.trim().is_empty() {
			bail!("Invalid series identity");
		}
		Ok(self.id.to_string())
	}

	pub fn into_details(self, base_url: &str) -> Result<Manga> {
		let basic = Manga::try_from(&self)?;
		let mut tags: Vec<String> = self
			.genres
			.unwrap_or_default()
			.into_iter()
			.map(|g| g.name)
			.collect();
		tags.sort_unstable();
		let content_rating = if self.is_18_plus == Some(true)
			|| tags.iter().any(|tag| {
				matches!(
					tag.to_lowercase().as_str(),
					"adult" | "hentai" | "smut" | "yaoi" | "pornographic"
				)
			}) {
			ContentRating::NSFW
		} else if tags
			.iter()
			.any(|tag| matches!(tag.to_lowercase().as_str(), "ecchi" | "mature"))
		{
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		};
		Ok(Manga {
			authors: self
				.author
				.map(|s| s.trim().into())
				.filter(|s: &String| !s.is_empty())
				.map(|s| vec![s]),
			artists: self
				.artist
				.map(|s| s.trim().into())
				.filter(|s: &String| !s.is_empty())
				.map(|s| vec![s]),
			description: self
				.post_content
				.and_then(|s| Html::parse(format!("<div id=description>{s}</div>")).ok())
				.and_then(|html| html.select_first("#description"))
				.and_then(|e| e.text_with_newlines())
				.map(|s| s.trim().escape_markdown())
				.filter(|s| !s.is_empty()),
			url: Some(format!("{base_url}/series/{}", self.slug)),
			tags: Some(tags),
			content_rating,
			status: match self.series_status.as_deref() {
				Some("ONGOING" | "COMING_SOON" | "MASS_RELEASED") => MangaStatus::Ongoing,
				Some("COMPLETED" | "ONE_SHOT") => MangaStatus::Completed,
				Some("CANCELLED" | "DROPPED") => MangaStatus::Cancelled,
				Some("HIATUS") => MangaStatus::Hiatus,
				_ => MangaStatus::Unknown,
			},
			viewer: match self.series_type.as_deref() {
				Some("MANGA") => Viewer::RightToLeft,
				Some(
					"MANHWA"
					| "MANHUA"
					| "WEBTOON"
					| "MANHWA_BL"
					| "MANHUA_BL"
					| "MANHWA_PLUS_19"
					| "BL_SIN_CENSURA"
					| "PLUS_19_SIN_CENSURA",
				) => Viewer::Webtoon,
				_ => Viewer::Unknown,
			},
			..basic
		})
	}
}

impl TryFrom<&Post> for Manga {
	type Error = aidoku::AidokuError;
	fn try_from(value: &Post) -> Result<Self> {
		Ok(Self {
			key: value.key()?,
			title: value.post_title.trim().into(),
			cover: value
				.featured_image
				.clone()
				.filter(|s| !s.trim().is_empty()),
			..Default::default()
		})
	}
}

impl IkenChapter {
	pub fn locked(&self) -> bool {
		self.is_locked == Some(true)
			|| self.is_time_locked == Some(true)
			|| self.is_permanently_locked == Some(true)
			|| self.is_locked_by_coins == Some(true)
			|| self.is_short_link_locked == Some(true)
			|| self.is_accessible == Some(false)
			|| (self.is_accessible != Some(true)
				&& self.price.is_some_and(|p| p > 0.)
				&& self.is_purchased != Some(true))
	}

	pub fn into_chapter(self, base_url: &str, manga_slug: Option<&str>) -> Result<Chapter> {
		if self.id <= 0 || self.slug.trim().is_empty() {
			bail!("Invalid chapter identity");
		}
		let locked = self.locked();
		let slug = manga_slug.or(self.manga_post.as_ref().map(|p| p.slug.as_str()));
		Ok(Chapter {
			key: self.id.to_string(),
			title: self
				.title
				.map(|s| s.trim().into())
				.filter(|s: &String| !s.is_empty()),
			chapter_number: Some(self.number),
			date_uploaded: self.created_at.and_then(|d| {
				parse_date_with_options(d, "yyyy-MM-dd'T'HH:mm:ss.SSS'Z'", "en_US_POSIX", "UTC")
			}),
			scanlators: self
				.team
				.or(self.created_by)
				.map(|a| a.name.trim().into())
				.filter(|s: &String| !s.is_empty())
				.map(|s| vec![s]),
			url: slug.map(|s| format!("{base_url}/series/{s}/{}", self.slug)),
			locked,
			..Default::default()
		})
	}
}
