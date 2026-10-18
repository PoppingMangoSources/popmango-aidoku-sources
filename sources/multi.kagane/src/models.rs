use aidoku::{
	Chapter, Manga, MangaWithChapter,
	alloc::{String, Vec, format, vec},
	imports::std::parse_date,
};
use serde::Deserialize;
use serde_json::Value;

use crate::helpers::{content_rating, format_title};

fn default_true() -> bool {
	true
}

#[derive(Deserialize)]
pub struct SearchResponse {
	#[serde(default)]
	pub content: Vec<SeriesSummary>,
	#[serde(default = "default_true")]
	pub last: bool,
}

#[derive(Deserialize)]
pub struct TrendingResponse {
	#[serde(default)]
	pub items: Vec<TrendingItem>,
}

#[derive(Deserialize)]
pub struct TrendingItem {
	#[serde(default)]
	pub hidden_by_filter: bool,
	pub series: SeriesSummary,
}

#[derive(Deserialize)]
pub struct SeriesSummary {
	pub series_id: String,
	pub title: String,
	pub cover_image_id: Option<String>,
	pub source_id: Option<String>,
	pub age_rating: Option<String>,
	pub is_pornographic: Option<bool>,
	pub translated_language: Option<String>,
	#[serde(default)]
	pub latest_chapters: Vec<LatestChapter>,
}

#[derive(Deserialize)]
pub struct LatestChapter {
	pub book_id: String,
	pub title: Option<String>,
	pub chapter_no: Option<String>,
	pub volume_no: Option<String>,
	pub created_at: Option<String>,
}

impl SeriesSummary {
	/// `sources` is empty unless the source's name is shown in titles.
	pub fn into_manga(self, base_url: &str, sources: &[UploadSource]) -> Manga {
		let source = self
			.source_id
			.as_deref()
			.and_then(|id| sources.iter().find(|source| source.source_id == id));
		Manga {
			content_rating: content_rating(self.age_rating.as_deref(), self.is_pornographic),
			title: format_title(
				&self.title,
				source.map(|source| source.title.as_str()),
				None,
			),
			cover: self
				.cover_image_id
				.map(|id| format!("{base_url}/api/v2/image/{id}")),
			url: Some(format!("{base_url}/series/{}", self.series_id)),
			key: self.series_id,
			..Default::default()
		}
	}

	/// The series with its newest chapter, or `None` when it has no chapters yet.
	pub fn into_manga_with_chapter(
		mut self,
		base_url: &str,
		sources: &[UploadSource],
	) -> Option<MangaWithChapter> {
		let book = self.latest_chapters.drain(..).next()?;
		Some(MangaWithChapter {
			chapter: Chapter {
				key: book.book_id,
				chapter_number: book.chapter_no.as_deref().and_then(|s| s.parse().ok()),
				volume_number: book.volume_no.as_deref().and_then(|s| s.parse().ok()),
				title: book
					.title
					.as_deref()
					.map(str::trim)
					.filter(|title| !title.is_empty())
					.map(Into::into),
				date_uploaded: book.created_at.as_deref().and_then(|s| {
					let s = s.split_once('.').map_or(s, |(b, _)| b);
					parse_date(format!("{s}Z"), "yyyy-MM-dd'T'HH:mm:ss'Z'")
				}),
				..Default::default()
			},
			manga: self.into_manga(base_url, sources),
		})
	}
}

#[derive(Deserialize)]
pub struct SeriesDetail {
	pub title: String,
	pub description: Option<String>,
	pub upload_status: String,
	pub format: Option<String>,
	pub age_rating: Option<String>,
	pub is_pornographic: Option<bool>,
	pub source_id: Option<String>,
	pub edition_info: Option<String>,
	#[serde(default)]
	pub series_staff: Vec<StaffMember>,
	#[serde(default)]
	pub genres: Vec<GenreItem>,
	#[serde(default)]
	pub tags: Vec<TagItem>,
	#[serde(default)]
	pub series_covers: Vec<CoverItem>,
}

#[derive(Deserialize)]
pub struct StaffMember {
	pub name: String,
	pub role: String,
}

#[derive(Deserialize)]
pub struct GenreItem {
	pub genre_name: String,
	#[serde(default)]
	pub is_spoiler: bool,
}

#[derive(Deserialize)]
pub struct TagItem {
	pub tag_name: String,
	#[serde(default)]
	pub is_spoiler: bool,
}

#[derive(Deserialize)]
pub struct GroupItem {
	pub title: String,
}

#[derive(Deserialize)]
pub struct CoverItem {
	pub image_id: String,
}

#[derive(Deserialize)]
pub struct GenreEntry {
	pub id: String,
	pub genre_name: String,
}

#[derive(Deserialize)]
pub struct TagEntry {
	pub id: String,
	pub tag_name: String,
}

#[derive(Deserialize)]
pub struct SourceList {
	#[serde(default)]
	pub sources: Vec<UploadSource>,
}

#[derive(Deserialize)]
pub struct UploadSource {
	pub source_id: String,
	pub title: String,
	pub source_type: Option<String>,
}

#[derive(Deserialize)]
pub struct StaffResults {
	#[serde(default)]
	pub content: Vec<StaffResult>,
}

#[derive(Deserialize)]
pub struct StaffResult {
	pub staff_id: String,
	pub name: String,
}

#[derive(Deserialize)]
pub struct GroupResults {
	#[serde(default)]
	pub items: Vec<GroupResult>,
}

#[derive(Deserialize)]
pub struct GroupResult {
	pub group_id: String,
	pub title: String,
}

/// The compact chapter list, where each book is a row of values in the order `fields`
/// names and its groups are positions in `groups`.
#[derive(Deserialize)]
pub struct BookIndex {
	pub fields: Vec<String>,
	#[serde(default)]
	pub groups: Vec<GroupItem>,
	#[serde(default)]
	pub books: Vec<Vec<Value>>,
}

fn cell(row: &[Value], at: Option<usize>) -> Option<&Value> {
	row.get(at?)
}

fn text(row: &[Value], at: Option<usize>) -> Option<&str> {
	cell(row, at)?.as_str()
}

impl BookIndex {
	/// Newest first by the index's own reading order, since chapter numbers can restart.
	/// `uploader` is the series' source, which names an upload better than its groups do.
	pub fn into_chapters(
		self,
		base_url: &str,
		series_key: &str,
		uploader: Option<&str>,
	) -> Vec<Chapter> {
		let column = |name: &str| self.fields.iter().position(|field| field == name);
		let [id, title, sort, chapter, volume, released, group_ix] = [
			"book_id",
			"title",
			"sort_no",
			"chapter_no",
			"volume_no",
			"released_at",
			"group_ix",
		]
		.map(column);
		let Self { groups, books, .. } = self;

		let mut chapters: Vec<(f64, Chapter)> = books
			.iter()
			.filter_map(|row| {
				let key = text(row, id)?;
				let name = text(row, title).map(str::trim).unwrap_or_default();
				let scanlators: Vec<String> = match uploader {
					Some(uploader) => vec![uploader.into()],
					None => cell(row, group_ix)
						.and_then(Value::as_array)
						.into_iter()
						.flatten()
						.filter_map(|at| groups.get(at.as_u64()? as usize))
						.map(|group| group.title.clone())
						.collect(),
				};
				let chapter = Chapter {
					key: key.into(),
					title: (!name.is_empty()).then(|| name.into()),
					chapter_number: text(row, chapter).and_then(|number| number.parse().ok()),
					volume_number: text(row, volume).and_then(|number| number.parse().ok()),
					date_uploaded: cell(row, released).and_then(Value::as_i64),
					scanlators: (!scanlators.is_empty()).then_some(scanlators),
					url: Some(format!("{base_url}/series/{series_key}/reader/{key}")),
					..Default::default()
				};
				Some((
					cell(row, sort).and_then(Value::as_f64).unwrap_or_default(),
					chapter,
				))
			})
			.collect();
		chapters.sort_by(|a, b| b.0.total_cmp(&a.0));
		chapters.into_iter().map(|(_, chapter)| chapter).collect()
	}
}

#[derive(Deserialize)]
pub struct IntegrityResponse {
	pub token: String,
}

#[derive(Deserialize)]
pub struct ChallengeResponse {
	pub access_token: String,
	pub cache_url: String,
	pub manifest: Option<ManifestData>,
}

#[derive(Deserialize)]
pub struct ManifestData {
	#[serde(default)]
	pub pages: Vec<PageData>,
}

#[derive(Deserialize)]
pub struct PageData {
	pub page_no: i32,
	pub page_id: String,
	pub ext: Option<String>,
}
