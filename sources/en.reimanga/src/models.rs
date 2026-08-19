use aidoku::alloc::{String, Vec};
use serde::Deserialize;

/// A genre/tag/author entry that the API sends either as a bare string or as an
/// object carrying a `name`.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum TagValue {
	Str(String),
	Obj(ApiTag),
}

impl TagValue {
	pub fn name(&self) -> Option<&str> {
		match self {
			TagValue::Str(s) => Some(s.as_str()),
			TagValue::Obj(t) => t.name.as_deref(),
		}
	}
}

#[derive(Deserialize)]
pub struct ApiTag {
	pub name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ApiManga {
	pub id: i64,
	pub title: Option<String>,
	pub name: Option<String>,
	pub alt_title: Option<String>,
	pub name_url: Option<String>,
	pub description: Option<String>,
	pub cover_url: Option<String>,
	pub completed: Option<i64>,
	pub view_count: Option<i64>,
	pub is_adult: Option<i64>,
	pub chapter_updated_at: Option<String>,
	pub release_date: Option<String>,
	pub updated_at: Option<String>,
	pub recent_reads: Option<i64>,
	pub genres: Option<Vec<TagValue>>,
	pub genre_slugs: Option<String>,
	pub tags: Option<Vec<TagValue>>,
	pub authors: Option<Vec<TagValue>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ApiPagination {
	#[serde(rename = "currentPage")]
	pub current_page: Option<i64>,
	#[serde(rename = "totalPages")]
	pub total_pages: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ApiMangaList {
	pub data: Option<Vec<ApiManga>>,
	pub pagination: Option<ApiPagination>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ApiMangaDetails {
	pub manga: Option<ApiManga>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FlightChapter {
	pub id: Option<i64>,
	pub name: Option<String>,
	#[serde(rename = "uploadDate")]
	pub upload_date: Option<String>,
	#[serde(rename = "updatedAt")]
	pub updated_at: Option<String>,
	#[serde(rename = "createdAt")]
	pub created_at: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FlightChapterList {
	pub chapters: Option<Vec<FlightChapter>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FlightImage {
	pub page_number: Option<i64>,
	pub image_url: Option<String>,
	pub url: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct FlightImages {
	pub images: Option<Vec<FlightImage>>,
}
