use crate::{
	api::{BASE_URL, CDN_URL},
	helpers::{
		clean_chapter_title, demographic_name, media_url, non_empty, parse_chapter_number,
		parse_content_rating, parse_iso_date, parse_status, plain_synopsis, viewer,
	},
};
use aidoku::{
	AidokuError, Chapter, Manga, MangaStatus, Result,
	alloc::{String, Vec, format},
	prelude::*,
};
use serde::{Deserialize, Serialize};

const MAX_TERMS: usize = 20;

#[derive(Serialize, Deserialize)]
pub struct Rpc<T> {
	pub json: T,
}

#[derive(Serialize)]
pub struct Empty {}

#[derive(Serialize)]
pub struct SlugInput<'a> {
	pub slug: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInput<'a> {
	pub release_id: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct IndexInput<'a> {
	pub work_id: &'a str,
	pub stream_id: &'a str,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub edition_id: Option<&'a str>,
	pub lang: &'a str,
	pub order: &'static str,
	pub limit: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cursor: Option<&'a str>,
}

// `chapters/select` requires `editionId` to be present (nullable, not optional).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SelectInput<'a> {
	pub work_id: &'a str,
	pub stream_id: &'a str,
	pub edition_id: Option<&'a str>,
	pub chapter_ids: &'a [String],
	pub language: &'a str,
	pub preference_version: i32,
}

#[derive(Deserialize)]
pub struct ErrorResponse {
	pub error: ErrorBody,
}

#[derive(Deserialize)]
pub struct ErrorBody {
	pub message: Option<String>,
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

	pub fn finalize(&mut self) {
		dedup_keep_order(&mut self.include);
		self.include.truncate(MAX_TERMS);
		let include = &self.include;
		self.exclude.retain(|slug| !include.contains(slug));
		dedup_keep_order(&mut self.exclude);
		self.exclude.truncate(MAX_TERMS);
	}
}

pub fn dedup_keep_order(values: &mut Vec<String>) {
	let mut seen = Vec::with_capacity(values.len());
	values.retain(|value| {
		if seen.contains(value) {
			false
		} else {
			seen.push(value.clone());
			true
		}
	});
}

#[derive(Default, Serialize)]
pub struct Years {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub from: Option<i32>,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub to: Option<i32>,
}

#[derive(Deserialize)]
pub struct TermOption {
	pub slug: String,
	pub name: String,
	pub group: String,
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
	medium: Option<String>,
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
#[serde(rename_all = "camelCase")]
pub struct HomeData {
	pub spotlight: Section<Vec<SpotlightItem>>,
	pub latest: Section<Vec<LatestEntry>>,
	pub new_on_site: Section<Vec<Work>>,
	pub finished: Section<Vec<Work>>,
	pub newly_published: Section<Vec<Work>>,
	#[serde(default)]
	pub trending: Vec<Trend>,
}

#[derive(Deserialize)]
pub struct Section<T> {
	pub value: Option<T>,
}

#[derive(Deserialize)]
pub struct SpotlightItem {
	pub card: Option<Work>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Trend {
	slug: String,
	title: Option<String>,
	medium: Option<String>,
	origin: Option<String>,
	content_rating: Option<String>,
}

#[derive(Deserialize)]
pub struct LatestEntry {
	pub card: Option<Work>,
	#[serde(default)]
	pub units: Vec<LatestUnit>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LatestUnit {
	release_id: Option<String>,
	label: String,
	title: Option<String>,
	language: String,
	at: Option<String>,
}

#[derive(Deserialize)]
pub struct WorkDetail {
	pub slug: String,
	pub canonical_title: String,
	pub medium: String,
	pub origin: Option<String>,
	pub layout: Option<String>,
	pub demographic: Option<String>,
	pub publication_status: String,
	pub content_rating: String,
	pub synopsis_md: Option<String>,
	pub metadata_version: String,
	#[serde(default)]
	pub credits: Vec<Credit>,
	#[serde(default)]
	pub terms: Vec<WorkTerm>,
	pub reading: Reading,
}

#[derive(Deserialize)]
pub struct Credit {
	pub name: String,
	pub role: String,
}

#[derive(Deserialize)]
pub struct WorkTerm {
	pub group: String,
	pub name: String,
	#[serde(default)]
	pub spoiler: bool,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reading {
	pub work_id: String,
	#[serde(default)]
	pub streams: Vec<Stream>,
	#[serde(default)]
	pub languages: Vec<Scope>,
	#[serde(default)]
	pub groups: Vec<Group>,
	pub default_scope: Option<Scope>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Stream {
	pub id: String,
	#[serde(default)]
	pub is_default: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
	pub stream_id: String,
	pub edition_id: Option<String>,
	pub language: String,
	#[serde(default)]
	pub readable_units: i32,
}

#[derive(Deserialize)]
pub struct Group {
	pub id: String,
	pub name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPage {
	pub items: Vec<ChapterUnit>,
	pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterUnit {
	pub id: String,
	pub number: Option<String>,
	pub label: String,
	pub title: Option<String>,
	pub volume: Option<String>,
	#[serde(default)]
	pub readable_candidate_count: Option<i32>,
	pub selection: Option<Selection>,
}

#[derive(Deserialize)]
pub struct SelectPage {
	pub items: Vec<Selection>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Selection {
	#[serde(default)]
	pub chapter_id: String,
	pub selected_release_id: Option<String>,
	#[serde(default)]
	pub candidates: Vec<Release>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
	pub id: String,
	#[serde(default)]
	pub group_ids: Vec<String>,
	pub content_kind: String,
	pub published_at: String,
	pub readable_at: Option<String>,
	pub is_readable: bool,
	pub title: Option<String>,
	pub label: String,
}

#[derive(Deserialize)]
pub struct Bootstrap {
	pub kind: String,
	pub manifest: Option<Manifest>,
}

#[derive(Deserialize)]
pub struct Manifest {
	pub pages: Vec<PageImage>,
}

#[derive(Clone, Deserialize)]
pub struct PageImage {
	pub ord: i32,
	pub url: String,
}

impl Work {
	pub fn is_comic(&self) -> bool {
		self.medium
			.as_deref()
			.is_none_or(|medium| medium == "comic")
	}

	pub fn allowed(
		&self,
		ratings: &[String],
		origins: Option<&[String]>,
		languages: &[String],
	) -> bool {
		self.is_comic()
			&& self
				.content_rating
				.as_ref()
				.is_some_and(|rating| ratings.contains(rating))
			&& origins.is_none_or(|origins| {
				self.origin
					.as_ref()
					.is_some_and(|origin| origins.contains(origin))
			}) && (self.languages.is_empty()
			|| self
				.languages
				.iter()
				.any(|entry| languages.contains(&entry.language)))
	}

	pub fn into_details(mut self) -> Result<Manga> {
		let description = self.synopsis.take().as_deref().and_then(plain_synopsis);
		let viewer = viewer(self.layout.as_deref(), self.origin.as_deref(), &[]);
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
		if !work.is_comic() {
			bail!("OniSaga novel entries are not supported");
		}
		let title = work
			.title
			.and_then(non_empty)
			.ok_or_else(|| error!("OniSaga returned a title without a name"))?;
		if work.slug.is_empty() {
			bail!("OniSaga returned a title without a slug");
		}
		Ok(Manga {
			key: work.slug,
			title,
			cover: work.cover_url,
			status: work
				.publication_status
				.as_deref()
				.map_or(MangaStatus::Unknown, parse_status),
			content_rating: parse_content_rating(work.content_rating.as_deref()),
			..Default::default()
		})
	}
}

impl Trend {
	pub fn allowed(&self, ratings: &[String], origins: Option<&[String]>) -> bool {
		self.medium
			.as_deref()
			.is_none_or(|medium| medium == "comic")
			&& self
				.content_rating
				.as_ref()
				.is_none_or(|rating| ratings.contains(rating))
			&& origins.is_none_or(|origins| {
				self.origin
					.as_ref()
					.is_none_or(|origin| origins.contains(origin))
			})
	}
}

impl TryFrom<Trend> for Manga {
	type Error = AidokuError;

	fn try_from(trend: Trend) -> Result<Self> {
		if trend
			.medium
			.as_deref()
			.is_some_and(|medium| medium != "comic")
		{
			bail!("OniSaga novel entries are not supported");
		}
		let title = trend
			.title
			.and_then(non_empty)
			.ok_or_else(|| error!("OniSaga returned a trending entry without a title"))?;
		if trend.slug.is_empty() {
			bail!("OniSaga returned a trending entry without a slug");
		}
		let cover = Some(format!("{CDN_URL}/api/covers/{}", trend.slug));
		Ok(Manga {
			key: trend.slug,
			title,
			cover,
			content_rating: parse_content_rating(trend.content_rating.as_deref()),
			..Default::default()
		})
	}
}

impl LatestUnit {
	pub fn into_chapter(self, languages: &[String]) -> Option<Chapter> {
		if !languages.contains(&self.language) {
			return None;
		}
		let key = self.release_id.filter(|id| !id.is_empty())?;
		let chapter_number = parse_chapter_number(&self.label);
		let title = clean_chapter_title(self.title, &self.label, chapter_number);
		Some(Chapter {
			key,
			title,
			chapter_number,
			date_uploaded: self.at.as_deref().and_then(parse_iso_date),
			language: Some(self.language),
			..Default::default()
		})
	}
}

impl WorkDetail {
	pub fn is_comic(&self) -> bool {
		self.medium == "comic"
	}

	pub fn into_basic_manga(self) -> Result<Manga> {
		if !self.is_comic() {
			bail!("OniSaga novel entries are not supported");
		}
		let Some(title) = non_empty(self.canonical_title) else {
			bail!("OniSaga returned an incomplete title");
		};
		if self.slug.is_empty() {
			bail!("OniSaga returned an incomplete title");
		}
		let cover = Some(if self.metadata_version.is_empty() {
			format!("{CDN_URL}/api/covers/{}", self.slug)
		} else {
			format!(
				"{CDN_URL}/api/covers/{}?mv={}",
				self.slug, self.metadata_version
			)
		});
		Ok(Manga {
			key: self.slug,
			title,
			cover,
			status: parse_status(&self.publication_status),
			content_rating: parse_content_rating(Some(&self.content_rating)),
			..Default::default()
		})
	}
}

impl TryFrom<WorkDetail> for Manga {
	type Error = AidokuError;

	fn try_from(mut detail: WorkDetail) -> Result<Self> {
		let mut authors = Vec::new();
		let mut artists = Vec::new();
		for (name, role) in core::mem::take(&mut detail.credits)
			.into_iter()
			.filter_map(|credit| non_empty(credit.name).map(|name| (name, credit.role)))
		{
			let is_author = matches!(
				role.as_str(),
				"story" | "author" | "story_and_art" | "original_creator"
			);
			let is_artist = matches!(
				role.as_str(),
				"art" | "illustrator" | "story_and_art" | "character_design"
			);
			match (is_author, is_artist) {
				(true, true) => {
					if !authors.contains(&name) {
						authors.push(name.clone());
					}
					if !artists.contains(&name) {
						artists.push(name);
					}
				}
				(true, false) if !authors.contains(&name) => authors.push(name),
				(false, true) if !artists.contains(&name) => artists.push(name),
				_ => {}
			}
		}

		let mut tags = core::mem::take(&mut detail.terms)
			.into_iter()
			.filter(|term| {
				!term.spoiler
					&& matches!(
						term.group.as_str(),
						"genre" | "theme" | "demographic" | "tag"
					)
			})
			.filter_map(|term| non_empty(term.name))
			.collect::<Vec<_>>();
		if let Some(demographic) = detail
			.demographic
			.take()
			.as_deref()
			.and_then(demographic_name)
		{
			tags.push(demographic.into());
		}
		tags.sort_unstable();
		tags.dedup();

		let viewer = viewer(detail.layout.as_deref(), detail.origin.as_deref(), &tags);
		let description = detail.synopsis_md.take().and_then(non_empty);
		let basic = detail.into_basic_manga()?;
		let url = Some(format!("{BASE_URL}/title/{}", basic.key));
		Ok(Manga {
			authors: (!authors.is_empty()).then_some(authors),
			artists: (!artists.is_empty()).then_some(artists),
			description,
			url,
			tags: (!tags.is_empty()).then_some(tags),
			viewer,
			..basic
		})
	}
}

impl Reading {
	// Prefer the title's default stream when a language appears in more than one numbering.
	pub fn language_scopes(&self, languages: &[String]) -> Vec<Scope> {
		let default_stream = self
			.default_scope
			.as_ref()
			.map(|scope| scope.stream_id.as_str())
			.or_else(|| {
				self.streams
					.iter()
					.find(|stream| stream.is_default)
					.map(|stream| stream.id.as_str())
			});
		languages
			.iter()
			.filter_map(|language| {
				self.languages
					.iter()
					.filter(|scope| scope.language == *language && scope.readable_units > 0)
					.max_by_key(|scope| {
						let preferred = default_stream == Some(scope.stream_id.as_str())
							|| self.default_scope.as_ref().is_some_and(|def| {
								def.stream_id == scope.stream_id
									&& def.edition_id == scope.edition_id
									&& def.language == scope.language
							});
						(preferred, scope.readable_units)
					})
					.cloned()
			})
			.collect()
	}
}

impl ChapterUnit {
	pub fn into_chapters(
		self,
		slug: &str,
		language: &str,
		groups: &[Group],
		fallback: Option<Selection>,
	) -> Vec<Chapter> {
		let Some(selection) = self.selection.or(fallback) else {
			return Vec::new();
		};
		let chapter_number = self
			.number
			.as_deref()
			.and_then(|num| num.parse::<f32>().ok())
			.or_else(|| parse_chapter_number(&self.label));
		let volume_number = self
			.volume
			.as_deref()
			.and_then(|vol| vol.parse::<f32>().ok());
		let unit_label = self.label;
		let unit_title = self.title;
		let selected = selection.selected_release_id.as_deref();

		let mut releases = selection
			.candidates
			.into_iter()
			.filter(|release| {
				release.is_readable && release.content_kind == "images" && !release.id.is_empty()
			})
			.collect::<Vec<_>>();
		if let Some(selected_id) = selected
			&& let Some(pos) = releases
				.iter()
				.position(|release| release.id == selected_id)
			&& pos > 0
		{
			let chosen = releases.remove(pos);
			releases.insert(0, chosen);
		}

		releases
			.into_iter()
			.map(|release| {
				let scanlators = release
					.group_ids
					.iter()
					.filter_map(|id| {
						let name = groups.iter().find(|group| group.id == *id)?.name.trim();
						(!name.is_empty()).then(|| name.into())
					})
					.collect::<Vec<String>>();
				let title = clean_chapter_title(
					release.title.or_else(|| unit_title.clone()),
					if release.label.is_empty() {
						&unit_label
					} else {
						&release.label
					},
					chapter_number,
				);
				let date_uploaded = release
					.readable_at
					.as_deref()
					.or(Some(release.published_at.as_str()))
					.and_then(parse_iso_date);
				let url = Some(format!("{BASE_URL}/read/{slug}/{}", release.id));
				Chapter {
					key: release.id,
					title,
					chapter_number,
					volume_number,
					date_uploaded,
					scanlators: (!scanlators.is_empty()).then_some(scanlators),
					url,
					language: Some(language.into()),
					..Default::default()
				}
			})
			.collect()
	}
}

impl Bootstrap {
	pub fn into_pages(self) -> Result<Vec<PageImage>> {
		if self.kind == "text" {
			bail!("This OniSaga chapter is text, which this source cannot show");
		}
		let mut pages = self
			.manifest
			.map(|manifest| manifest.pages)
			.unwrap_or_default();
		pages.retain(|page| !page.url.is_empty());
		if pages.is_empty() {
			bail!("OniSaga has no pages for this chapter");
		}
		pages.sort_unstable_by_key(|page| page.ord);
		for page in &mut pages {
			let url = core::mem::take(&mut page.url);
			page.url = media_url(url);
		}
		Ok(pages)
	}
}
