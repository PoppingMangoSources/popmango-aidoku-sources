use crate::{
	api::{BASE_URL, CDN_URL},
	helpers::{chapter_number, content_rating, status, viewer},
};
use aidoku::{
	AidokuError, Chapter, ContentRating, Manga, MangaStatus, Result,
	alloc::{String, Vec},
	imports::std::parse_date_with_options,
	prelude::*,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Rpc<T> {
	pub json: T,
}

#[derive(Default, Serialize)]
pub struct Terms {
	pub include: Vec<String>,
	pub exclude: Vec<String>,
}

impl Terms {
	pub fn is_empty(&self) -> bool {
		self.include.is_empty() && self.exclude.is_empty()
	}
}

#[derive(Default, Serialize)]
pub struct Years {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub from: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub to: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowsePage {
	pub items: Vec<Work>,
	pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Work {
	slug: String,
	title: Option<String>,
	cover_url: Option<String>,
	origin: Option<String>,
	layout: Option<String>,
	publication_status: Option<String>,
	content_rating: Option<String>,
	synopsis: Option<String>,
	#[serde(default)]
	languages: Vec<WorkLanguage>,
}

#[derive(Deserialize)]
struct WorkLanguage {
	language: String,
}

#[derive(Deserialize)]
pub struct Spotlight {
	pub thumbs: Vec<Work>,
}

#[derive(Deserialize)]
pub struct LatestUpdates {
	pub entries: Vec<LatestEntry>,
}

#[derive(Deserialize)]
pub struct LatestEntry {
	pub card: Work,
	pub units: Vec<Unit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Unit {
	release_id: Option<String>,
	label: String,
	title: Option<String>,
	language: String,
	at: Option<String>,
}

#[derive(Deserialize)]
pub struct Reading {
	pub alternatives: Vec<Alternative>,
}

#[derive(Deserialize)]
pub struct Alternative {
	pub href: String,
	pub current: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GatedInput<'a> {
	pub slug: &'a str,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub release_id: Option<&'a str>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatedTitle {
	title: String,
	synopsis: Option<String>,
	publication_status: Option<String>,
	language: String,
	units: Vec<GatedUnit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct GatedUnit {
	label: String,
	title: Option<String>,
	release_id: Option<String>,
}

#[derive(Deserialize)]
pub struct GatedReader {
	bootstrap: Bootstrap,
}

#[derive(Deserialize)]
struct Bootstrap {
	manifest: Manifest,
}

#[derive(Deserialize)]
struct Manifest {
	pages: Vec<PageImage>,
}

#[derive(Deserialize)]
struct PageImage {
	url: String,
}

impl GatedTitle {
	// The API lists one language, oldest first.
	pub fn into_manga(self, key: &str, languages: &[String]) -> Manga {
		let chapters = if languages.contains(&self.language) {
			self.units
				.into_iter()
				.rev()
				.filter_map(|unit| {
					Some(Chapter {
						url: Some(format!(
							"{BASE_URL}/read/{key}/{}",
							unit.release_id.as_ref()?
						)),
						key: unit.release_id?,
						chapter_number: chapter_number(&unit.label),
						title: unit.title,
						language: Some(self.language.clone()),
						..Default::default()
					})
				})
				.collect()
		} else {
			Vec::new()
		};
		Manga {
			key: key.into(),
			title: self.title,
			description: self.synopsis.filter(|synopsis| !synopsis.is_empty()),
			status: self
				.publication_status
				.as_deref()
				.map_or(MangaStatus::Unknown, status),
			content_rating: ContentRating::NSFW,
			url: Some(format!("{BASE_URL}/title/{key}")),
			chapters: Some(chapters),
			..Default::default()
		}
	}
}

impl GatedReader {
	// The image host refuses these pages; the age cookie only reaches the main host.
	pub fn into_urls(self) -> Vec<String> {
		self.bootstrap
			.manifest
			.pages
			.into_iter()
			.map(|page| page.url.replacen(CDN_URL, BASE_URL, 1))
			.collect()
	}
}

impl Work {
	pub fn allowed(
		&self,
		ratings: &[String],
		origins: Option<&[String]>,
		languages: &[String],
	) -> bool {
		self.content_rating
			.as_ref()
			.is_some_and(|rating| ratings.contains(rating))
			&& origins.is_none_or(|origins| {
				self.origin
					.as_ref()
					.is_some_and(|origin| origins.contains(origin))
			}) && self
			.languages
			.iter()
			.any(|language| languages.contains(&language.language))
	}

	pub fn into_details(mut self) -> Result<Manga> {
		let description = self.synopsis.take().filter(|synopsis| !synopsis.is_empty());
		let viewer = viewer(
			self.layout.as_deref() == Some("vertical_strip"),
			self.origin.as_deref(),
		);
		Ok(Manga {
			description,
			viewer,
			..self.try_into()?
		})
	}
}

impl TryFrom<Work> for Manga {
	type Error = AidokuError;

	fn try_from(work: Work) -> Result<Self> {
		let title = work
			.title
			.filter(|title| !title.is_empty())
			.ok_or_else(|| error!("OniSaga returned a title without a name"))?;
		Ok(Manga {
			key: work.slug,
			title,
			cover: work.cover_url,
			status: work
				.publication_status
				.as_deref()
				.map_or(MangaStatus::Unknown, status),
			content_rating: content_rating(work.content_rating.as_deref()),
			..Default::default()
		})
	}
}

impl Unit {
	// The first release in a language the reader chose.
	pub fn into_chapter(self, languages: &[String]) -> Option<Chapter> {
		if !languages.contains(&self.language) {
			return None;
		}
		Some(Chapter {
			key: self.release_id?,
			title: self.title,
			chapter_number: chapter_number(&self.label),
			date_uploaded: self.at.as_deref().and_then(parse_time),
			language: Some(self.language),
			..Default::default()
		})
	}
}

pub fn parse_time(time: &str) -> Option<i64> {
	parse_date_with_options(
		time,
		"yyyy-MM-dd'T'HH:mm:ss.SSSSSSXXX",
		"en_US_POSIX",
		"UTC",
	)
}
