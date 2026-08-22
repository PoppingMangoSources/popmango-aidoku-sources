use aidoku::alloc::{String, Vec};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GraphQlResponse<T> {
	pub data: Option<T>,
	#[serde(default)]
	pub errors: Vec<GraphQlError>,
}

#[derive(Deserialize)]
pub struct GraphQlError {
	pub message: String,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct NamedData {
	pub name: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct NamedNode {
	pub data: Option<NamedData>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Summary {
	pub text: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ComicData {
	pub id: String,
	pub name: String,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	pub demographics: Option<Vec<String>>,
	pub content_rating: Option<String>,
	pub genres: Option<Vec<String>>,
	pub tags: Option<Vec<String>>,
	pub author_nodes: Option<Vec<NamedNode>>,
	pub artist_nodes: Option<Vec<NamedNode>>,
	pub tag_nodes: Option<Vec<NamedNode>>,
	pub summary: Option<Summary>,
	pub url_path: Option<String>,
	pub url_cover: Option<String>,
	pub original_status: Option<String>,
	pub upload_status: Option<String>,
	pub read_direction: Option<String>,
	pub translated_language: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ComicNode {
	pub data: ComicData,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct BrowseResponse {
	pub get_comic_browse_items: Vec<ComicNode>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct OptionalComicNode {
	pub data: Option<ComicData>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUploadsItem {
	pub comic: Option<OptionalComicNode>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUploadsResult {
	pub before: Option<i64>,
	pub items: Vec<LatestUploadsItem>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUploadsResponse {
	#[serde(rename = "get_comic_latestUploads")]
	pub latest_uploads: Option<LatestUploadsResult>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ComicNodeResponse {
	#[serde(rename = "get_comicNode")]
	pub comic: Option<ComicNode>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ChapterData {
	pub id: String,
	pub db_status: Option<String>,
	pub serial: Option<f64>,
	pub cha_num: Option<f64>,
	pub dname: Option<String>,
	pub title: Option<String>,
	pub url_path: Option<String>,
	pub date_create: Option<i64>,
	pub date_modify: Option<i64>,
	pub date_public: Option<i64>,
	pub src_name: Option<String>,
	pub profile_nodes: Option<Vec<NamedNode>>,
	pub group_nodes: Option<Vec<NamedNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterNode {
	pub data: ChapterData,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Paging {
	pub pages: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterListResult {
	pub paging: Option<Paging>,
	pub items: Vec<ChapterNode>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterListResponse {
	#[serde(rename = "get_comic_chapterList_uniqList")]
	pub chapter_list: Option<ChapterListResult>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ChapterPageData {
	pub image_urls: Vec<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterPageNode {
	pub data: Option<ChapterPageData>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterPagesResponse {
	#[serde(rename = "get_chapterNode")]
	pub chapter: Option<ChapterPageNode>,
}
