use aidoku::alloc::{String, Vec};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
	pub data: Option<T>,
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
pub struct DateYmd {
	pub y: Option<i64>,
	pub m: Option<i64>,
	pub d: Option<i64>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct Summary {
	pub html: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ComicData {
	pub id: String,
	pub name: String,
	#[serde(rename = "altNames")]
	pub alt_names: Option<Vec<String>>,
	#[serde(rename = "originalLanguage")]
	pub original_language: Option<String>,
	#[serde(rename = "translatedLanguage")]
	pub translated_language: Option<String>,
	#[serde(rename = "originalStatus")]
	pub original_status: Option<String>,
	#[serde(rename = "uploadStatus")]
	pub upload_status: Option<String>,
	#[serde(rename = "originalPubFrom")]
	pub original_pub_from: Option<DateYmd>,
	#[serde(rename = "originalPubTill")]
	pub original_pub_till: Option<DateYmd>,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	pub demographics: Option<Vec<String>>,
	#[serde(rename = "contentRating")]
	pub content_rating: Option<String>,
	pub genres: Option<Vec<String>>,
	pub tags: Option<Vec<String>>,
	#[serde(rename = "authorNodes")]
	pub author_nodes: Option<Vec<NamedNode>>,
	#[serde(rename = "artistNodes")]
	pub artist_nodes: Option<Vec<NamedNode>>,
	#[serde(rename = "tagNodes")]
	pub tag_nodes: Option<Vec<NamedNode>>,
	pub summary: Option<Summary>,
	#[serde(rename = "urlPath")]
	pub url_path: Option<String>,
	#[serde(rename = "urlCover")]
	pub url_cover: Option<String>,
	pub sfw_result: Option<bool>,
	pub score_val: Option<f64>,
	pub chaps_normal: Option<i64>,
	#[serde(rename = "chapterNodes_last")]
	pub chapter_nodes_last: Option<Vec<ChapterNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ComicNode {
	pub data: ComicData,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterData {
	pub id: String,
	#[serde(rename = "dbStatus")]
	pub db_status: Option<String>,
	pub serial: Option<f64>,
	#[serde(rename = "chaNum")]
	pub cha_num: Option<f64>,
	pub dname: Option<String>,
	pub title: Option<String>,
	#[serde(rename = "urlPath")]
	pub url_path: Option<String>,
	#[serde(rename = "dateCreate")]
	pub date_create: Option<i64>,
	#[serde(rename = "dateModify")]
	pub date_modify: Option<i64>,
	#[serde(rename = "datePublic")]
	pub date_public: Option<i64>,
	#[serde(rename = "srcName")]
	pub src_name: Option<String>,
	#[serde(rename = "profileNodes")]
	pub profile_nodes: Option<Vec<NamedNode>>,
	#[serde(rename = "userNode")]
	pub user_node: Option<NamedNode>,
	#[serde(rename = "groupNodes")]
	pub group_nodes: Option<Vec<NamedNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterNode {
	pub data: ChapterData,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct BrowseResponse {
	pub get_comic_browse_items: Option<Vec<ComicNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ComicNodeResponse {
	#[serde(rename = "get_comicNode")]
	pub get_comic_node: Option<ComicNode>,
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
	pub items: Option<Vec<ChapterNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterListResponse {
	#[serde(rename = "get_comic_chapterList_uniqList")]
	pub chapter_list: Option<ChapterListResult>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterNodeData {
	#[serde(rename = "imageUrls")]
	pub image_urls: Option<Vec<String>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterNodeWrap {
	pub data: Option<ChapterNodeData>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct ChapterPagesResponse {
	#[serde(rename = "get_chapterNode")]
	pub get_chapter_node: Option<ChapterNodeWrap>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUploadItem {
	pub comic: Option<ComicNode>,
	pub chapters: Option<Vec<ChapterNode>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUploadsResult {
	pub before: Option<i64>,
	pub items: Option<Vec<LatestUploadItem>>,
}

#[derive(Deserialize, Default)]
#[serde(default)]
pub struct LatestUpdatesResponse {
	#[serde(rename = "get_comic_latestUploads")]
	pub latest_uploads: Option<LatestUploadsResult>,
}
