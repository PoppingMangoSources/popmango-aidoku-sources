use crate::{
	helpers::{PORNOGRAPHIC_GENRES, is_pornographic},
	models::{
		BrowseResponse, ChapterData, ChapterListResponse, ChapterPagesResponse, ComicData,
		ComicNodeResponse, GraphQlResponse, LatestUploadsResponse, LatestUploadsResult,
	},
	settings,
};
use aidoku::{
	Result,
	alloc::{String, Vec, string::ToString},
	imports::{
		defaults::defaults_get,
		net::{Request, Response},
	},
	prelude::*,
};
use serde::de::DeserializeOwned;

pub const PAGE_SIZE: i32 = 36;
pub const EXTENDED_PAGE_SIZE: i32 = 120;
const CHAPTER_PAGE_SIZE: i32 = 1000;

pub const SORT_IDS: &[&str] = &[
	"field_score",
	"field_update",
	"field_create",
	"field_name_asc",
	"field_name_desc",
	"field_chapter",
	"field_follow",
	"field_review",
	"field_comment",
	"views_d000",
	"views_d360",
	"views_d180",
	"views_d090",
	"views_d030",
	"views_d007",
	"views_h024",
	"views_h012",
	"views_h006",
	"views_h001",
];

const DEFAULT_TYPES: &[&str] = &["manga", "manhwa", "manhua"];
const DEFAULT_RATINGS: &[&str] = &["safe", "suggestive", "erotica", "pornographic"];

const BROWSE_QUERY: &str = r#"
query get_comic_browse_items($select: Comic_Browse_Select) {
  get_comic_browse_items(select: $select) {
    data {
      id name urlPath urlCover
      type contentRating genres
    }
  }
}
"#;

const LATEST_UPLOADS_QUERY: &str = r#"
query get_comic_latestUploads($select: Comic_LatestUploads_Select) {
  get_comic_latestUploads(select: $select) {
    before
    items {
      comic {
        data {
          id name urlPath urlCover translatedLanguage
          type contentRating genres
        }
      }
    }
  }
}
"#;

const COMIC_QUERY: &str = r#"
query get_comicNode($id: ID!) {
  get_comicNode(id: $id) {
    data {
      id name type demographics contentRating genres tags
      originalStatus uploadStatus readDirection translatedLanguage
      authorNodes { data { name } }
      artistNodes { data { name } }
      tagNodes { data { name } }
      summary { text }
      urlPath urlCover
    }
  }
}
"#;

const CHAPTERS_QUERY: &str = r#"
query get_comic_chapterList_uniqList($select: Select_Comic_ChapterList_UniqList) {
  get_comic_chapterList_uniqList(select: $select) {
    paging { pages }
    items {
      data {
        id dbStatus serial chaNum dname title urlPath
        dateCreate dateModify datePublic srcName
        profileNodes { data { name } }
        groupNodes { data { name } }
      }
    }
  }
}
"#;

const CHAPTER_PAGES_QUERY: &str = r#"
query get_chapterNode($id: ID!) {
  get_chapterNode(id: $id) { data { imageUrls } }
}
"#;

fn setting_list(key: &str, fallback: &[&str]) -> Vec<String> {
	let values = defaults_get::<Vec<String>>(key).unwrap_or_default();
	if values.is_empty() {
		fallback.iter().map(|value| (*value).into()).collect()
	} else {
		values
	}
}

pub struct BrowseParams {
	pub page: i32,
	pub size: i32,
	pub sortby: String,
	pub word: String,
	pub included_genres: Vec<String>,
	pub excluded_genres: Vec<String>,
	pub include_mode: String,
	pub exclude_mode: String,
	pub types: Vec<String>,
	pub demographics: Vec<String>,
	pub content_ratings: Vec<String>,
	pub original_languages: Vec<String>,
	pub translated_languages: Vec<String>,
	pub original_status: String,
	pub upload_status: String,
	pub chapter_count: String,
	pub year_min: Option<i64>,
	pub year_max: Option<i64>,
}

impl BrowseParams {
	pub fn new(sortby: &str, page: i32) -> Result<Self> {
		let mut excluded_genres = defaults_get::<Vec<String>>("excludedGenres").unwrap_or_default();
		for tag in defaults_get::<Vec<String>>("excludedTags").unwrap_or_default() {
			if !excluded_genres.contains(&tag) {
				excluded_genres.push(tag);
			}
		}
		Ok(Self {
			page,
			size: PAGE_SIZE,
			sortby: sortby.into(),
			word: String::new(),
			included_genres: Vec::new(),
			excluded_genres,
			include_mode: "and".into(),
			exclude_mode: "or".into(),
			types: setting_list("contentTypes", DEFAULT_TYPES),
			demographics: Vec::new(),
			content_ratings: setting_list("contentRatings", DEFAULT_RATINGS),
			original_languages: Vec::new(),
			translated_languages: settings::get_languages()?,
			original_status: String::new(),
			upload_status: String::new(),
			chapter_count: String::new(),
			year_min: None,
			year_max: None,
		})
	}

	fn allows_pornographic(&self) -> bool {
		self.content_ratings
			.iter()
			.any(|value| value == "pornographic")
	}

	fn select(&self) -> serde_json::Value {
		let mut excluded_genres: Vec<&str> =
			self.excluded_genres.iter().map(String::as_str).collect();
		if !self.allows_pornographic() {
			for genre in PORNOGRAPHIC_GENRES {
				if !excluded_genres.contains(genre) {
					excluded_genres.push(genre);
				}
			}
		}
		serde_json::json!({
			"where": "browse",
			"page": self.page,
			"size": self.size,
			"init": (self.page - 1) * self.size,
			"sortby": self.sortby,
			"word": self.word,
			"incOLangs": self.original_languages,
			"incTLangs": self.translated_languages,
			"incGenres": self.included_genres,
			"excGenres": excluded_genres,
			"incGenresMode": self.include_mode,
			"excGenresMode": self.exclude_mode,
			"incTypes": self.types,
			"incDemographics": self.demographics,
			"incContentRatings": self.content_ratings,
			"releaseYearMin": self.year_min,
			"releaseYearMax": self.year_max,
			"origStatus": (!self.original_status.is_empty()).then_some(&self.original_status),
			"siteStatus": (!self.upload_status.is_empty()).then_some(&self.upload_status),
			"chapCount": (!self.chapter_count.is_empty()).then_some(&self.chapter_count),
			"ignoreGlobalULangs": true,
			"ignoreGlobalGenres": true,
			"ignoreGlobalBlocks": true
		})
	}

	fn allows(&self, comic: &ComicData) -> bool {
		let rating = comic.content_rating.as_deref().unwrap_or("safe");
		let genres = comic.genres.as_deref().unwrap_or_default();
		self.types
			.iter()
			.any(|kind| Some(kind.as_str()) == comic.kind.as_deref())
			&& self.content_ratings.iter().any(|value| value == rating)
			&& !genres
				.iter()
				.any(|genre| self.excluded_genres.contains(genre))
			&& (self.allows_pornographic()
				|| !is_pornographic(comic.content_rating.as_deref(), comic.genres.as_deref()))
	}

	pub fn can_use_latest_uploads(&self) -> bool {
		self.sortby == "field_update"
			&& self.word.is_empty()
			&& self.included_genres.is_empty()
			&& self.demographics.is_empty()
			&& self.original_languages.is_empty()
			&& self.original_status.is_empty()
			&& self.upload_status.is_empty()
			&& self.chapter_count.is_empty()
			&& self.year_min.is_none()
			&& self.year_max.is_none()
	}
}

pub fn graphql_request(
	base_url: &str,
	query: &str,
	variables: serde_json::Value,
) -> Result<Request> {
	let languages = settings::get_languages()?;
	let accept_language = if languages.is_empty() {
		"en".into()
	} else {
		languages
			.iter()
			.map(|language| language.replace('_', "-"))
			.collect::<Vec<_>>()
			.join(",")
	};
	Ok(Request::post(format!("{base_url}/query/"))?
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.header("Accept-Language", &accept_language)
		.header("Referer", &format!("{base_url}/"))
		.header("Origin", base_url)
		.body(serde_json::json!({ "query": query.trim(), "variables": variables }).to_string()))
}

fn parse_graphql<T: DeserializeOwned>(response: Response) -> Result<T> {
	let response: GraphQlResponse<T> = response.get_json_owned()?;
	if let Some(data) = response.data {
		Ok(data)
	} else if let Some(error) = response.errors.into_iter().next() {
		bail!("XCOMIC: {}", error.message);
	} else {
		bail!("XCOMIC returned an empty response");
	}
}

fn graphql<T: DeserializeOwned>(
	base_url: &str,
	query: &str,
	variables: serde_json::Value,
) -> Result<T> {
	parse_graphql(graphql_request(base_url, query, variables)?.send()?)
}

pub fn browse_request(base_url: &str, params: &BrowseParams) -> Result<Request> {
	graphql_request(
		base_url,
		BROWSE_QUERY,
		serde_json::json!({ "select": params.select() }),
	)
}

pub fn latest_uploads_request(base_url: &str, before: Option<i64>) -> Result<Request> {
	graphql_request(
		base_url,
		LATEST_UPLOADS_QUERY,
		serde_json::json!({
			"select": {
				"size": EXTENDED_PAGE_SIZE,
				"before": before
			}
		}),
	)
}

pub fn parse_latest_uploads(
	response: Response,
	params: &BrowseParams,
) -> Result<(Vec<ComicData>, Option<i64>)> {
	let response: LatestUploadsResponse = parse_graphql(response)?;
	let LatestUploadsResult { before, items } = response.latest_uploads.unwrap_or_default();
	let mut seen: Vec<String> = Vec::new();
	let comics = items
		.into_iter()
		.filter_map(|item| item.comic.and_then(|node| node.data))
		.filter(|comic| {
			params.translated_languages.is_empty()
				|| comic
					.translated_language
					.as_ref()
					.is_some_and(|language| params.translated_languages.contains(language))
		})
		.filter(|comic| params.allows(comic))
		.filter(|comic| {
			comic
				.url_cover
				.as_deref()
				.is_some_and(|cover| !cover.trim().is_empty())
		})
		// The feed lists one entry per upload, so a comic repeats per new chapter.
		.filter(|comic| {
			let unseen = !seen.contains(&comic.id);
			if unseen {
				seen.push(comic.id.clone());
			}
			unseen
		})
		.collect();
	Ok((comics, before))
}

/// Returns surviving comics plus whether a full page came back. The flag must use
/// the unfiltered count, or local filtering would cut pagination short.
pub fn parse_browse(response: Response, params: &BrowseParams) -> Result<(Vec<ComicData>, bool)> {
	let response: BrowseResponse = parse_graphql(response)?;
	let items = response.get_comic_browse_items;
	let has_next_page = items.len() as i32 >= params.size;
	let comics = items
		.into_iter()
		.map(|node| node.data)
		.filter(|comic| params.allows(comic))
		.collect();
	Ok((comics, has_next_page))
}

pub fn fetch_comic(base_url: &str, id: &str) -> Result<ComicData> {
	let response: ComicNodeResponse =
		graphql(base_url, COMIC_QUERY, serde_json::json!({ "id": id }))?;
	response
		.comic
		.map(|node| node.data)
		.ok_or_else(|| error!("Manga not found"))
}

pub fn fetch_chapters(base_url: &str, comic_id: &str) -> Result<Vec<ChapterData>> {
	let first: ChapterListResponse =
		graphql(base_url, CHAPTERS_QUERY, chapter_variables(comic_id, 1))?;
	let pages = first
		.chapter_list
		.as_ref()
		.and_then(|result| result.paging.as_ref())
		.and_then(|paging| paging.pages)
		.unwrap_or(1);
	let mut chapters: Vec<ChapterData> = first
		.chapter_list
		.map(|result| result.items.into_iter().map(|node| node.data).collect())
		.unwrap_or_default();
	for page in 2..=pages {
		let response: ChapterListResponse = graphql(
			base_url,
			CHAPTERS_QUERY,
			chapter_variables(comic_id, page as i32),
		)?;
		if let Some(result) = response.chapter_list {
			chapters.extend(result.items.into_iter().map(|node| node.data));
		}
	}
	Ok(chapters)
}

fn chapter_variables(comic_id: &str, page: i32) -> serde_json::Value {
	serde_json::json!({
		"select": {
			"comic_id": comic_id,
			"page": page,
			"size": CHAPTER_PAGE_SIZE,
			"sortby": "chapter_desc"
		}
	})
}

pub fn fetch_page_urls(base_url: &str, chapter_id: &str) -> Result<Vec<String>> {
	let response: ChapterPagesResponse = graphql(
		base_url,
		CHAPTER_PAGES_QUERY,
		serde_json::json!({ "id": chapter_id }),
	)?;
	Ok(response
		.chapter
		.and_then(|node| node.data)
		.map(|data| data.image_urls)
		.unwrap_or_default())
}
