use crate::{
	helpers::{ADULT_GENRES, HENTAI, allows, spotlight_genre},
	models::{ErrorBody, SearchResponse},
	settings::{self, CONTENT_TYPES, RATINGS},
};
use aidoku::{
	MangaPageResult, Result,
	alloc::{String, Vec, format, string::ToString},
	helpers::uri::QueryParameters,
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;

pub const BASE_URL: &str = "https://atsu.moe";
pub const CDN_URL: &str = "https://cdn.atsu.moe/static";
pub const FEED_LIMIT: i32 = 24;
const SEARCH_LIMIT: i32 = 40;
const SEARCH_URL: &str = "https://atsu.moe/collections/manga/documents/search";
pub const FILTERS_URL: &str = "https://atsu.moe/api/explore/availableFilters";

// The site's own search weighting.
const QUERY_BY: &str = "title,englishTitle,otherNames,authors,acronyms";
const QUERY_BY_WEIGHTS: &str = "4,3,2,2,1";
const NUM_TYPOS: &str = "4,3,2,1,0";
const PREFIX: &str = "true,true,true,true,false";
const INFIX: &str = "off,off,fallback,off,off";
const CARD_FIELDS: &str =
	"id,title,poster,posterMedium,isAdult,mbContentRating,genreIds,medium,status";
pub const RATING_FIELDS: &str = "id,isAdult,mbContentRating,genreIds";
pub const FEATURED_FIELDS: &str =
	"id,synopsis,authors,status,tags,isAdult,mbContentRating,genreIds";

/// Search sorts in `filters.json` order. Relevance ranks a text search by match and anything
/// else as the site's default does.
const SORTS: [&str; 7] = [
	"views:desc",
	"views:desc",
	"trending:desc",
	"dateAdded:desc",
	"releaseDate:desc",
	"mbRating:desc",
	"title:asc",
];
const RELEVANCE: usize = 0;
const POPULARITY: usize = 1;
const TOP_RATED: usize = 5;

pub fn request(url: &str) -> Result<Request> {
	Ok(Request::get(url)?
		.header("Accept", "application/json")
		.header("Referer", &format!("{BASE_URL}/")))
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		match response.get_json_owned::<ErrorBody>() {
			Ok(body) => bail!("Atsumaru: {}", body.message),
			Err(_) => bail!("Atsumaru returned status {status}"),
		}
	}
	response.get_json_owned()
}

pub fn get<T: DeserializeOwned>(url: &str) -> Result<T> {
	parse(request(url)?.send()?)
}

fn quote(value: &str) -> String {
	format!("`{}`", value.replace('\\', "\\\\").replace('`', "\\`"))
}

fn list<S: AsRef<str>>(values: &[S]) -> String {
	let quoted = values
		.iter()
		.map(|value| quote(value.as_ref()))
		.collect::<Vec<_>>();
	format!("[{}]", quoted.join(","))
}

/// Genres the ratings leave out, standing in for titles the site hasn't rated.
fn excluded_rating_genres(ratings: &[String]) -> Vec<&'static str> {
	let mut genres = Vec::new();
	if !allows(ratings, "Pornographic") {
		genres.push(HENTAI);
	}
	if !allows(ratings, "Erotica") {
		genres.extend(ADULT_GENRES);
	}
	genres
}

fn split_types(types: &[String]) -> (Vec<&str>, bool) {
	let comics = types
		.iter()
		.map(String::as_str)
		.filter(|kind| *kind != "Novel")
		.collect();
	(comics, allows(types, "Novel"))
}

/// A home feed or one of its listings; `None` when the settings leave it nothing to show.
pub fn feed_url(id: &str, page: i32) -> Result<Option<String>> {
	let mut query = QueryParameters::new();
	match id {
		"popular" | "mostBookmarked" | "mostTalkedAbout" => query.push("timeframe", Some("weekly")),
		"genreSpotlight" => query.push("genre", Some(spotlight_genre())),
		"hotUpdates" | "recentlyUpdated" | "rising" | "hotArrivals" | "recentlyAdded"
		| "topRated" | "hiddenGems" | "bingeWorthy" | "mostPolarizing" => {}
		_ => bail!("Unknown listing"),
	}
	let types = settings::content_types();
	let ratings = settings::content_ratings();
	let adult_only = settings::adult_only();
	let (comics, novels) = split_types(&types);
	if (comics.is_empty() && !novels) || (ratings.is_empty() && !adult_only) {
		return Ok(None);
	}
	query.push("offset", Some(&((page - 1) * FEED_LIMIT).to_string()));
	query.push("limit", Some(&FEED_LIMIT.to_string()));
	// The feeds AND these together, so a comic type would drop novels of other types; with
	// novels on, the types are applied to the items instead.
	let mediums: Vec<&str> = [(!comics.is_empty(), "Comic"), (novels, "Novel")]
		.into_iter()
		.filter_map(|(wanted, medium)| wanted.then_some(medium))
		.collect();
	query.push("mediums", Some(&mediums.join(",")));
	if !novels {
		query.push("types", Some(&comics.join(",")));
	}
	let mut excluded = settings::hidden_genres();
	excluded.extend(settings::hidden_tags());
	if adult_only {
		query.push("adult", Some("1"));
	} else {
		// The feeds take genre and tag ids in one parameter; the two never share an id.
		excluded.extend(excluded_rating_genres(&ratings).into_iter().map(Into::into));
		query.push("contentRatings", Some(&ratings.join(",")));
	}
	if !excluded.is_empty() {
		query.push("excludedTags", Some(&excluded.join(",")));
	}
	Ok(Some(format!("{BASE_URL}/api/home2/{id}?{query}")))
}

pub fn lookup_url(ids: &[&str], fields: &str) -> String {
	let mut query = QueryParameters::new();
	query.push("q", Some("*"));
	query.push("query_by", Some("title"));
	query.push("include_fields", Some(fields));
	query.push("filter_by", Some(&format!("id:={}", list(ids))));
	query.push("per_page", Some(&ids.len().max(1).to_string()));
	format!("{SEARCH_URL}?{query}")
}

#[derive(Default)]
pub struct Search {
	pub query: Option<String>,
	pub sort: usize,
	pub genres: Vec<String>,
	pub excluded_genres: Vec<String>,
	pub any_genre: bool,
	pub tags: Vec<String>,
	pub excluded_tags: Vec<String>,
	pub any_tag: bool,
	pub demographics: Vec<String>,
	pub types: Vec<String>,
	pub statuses: Vec<String>,
	pub ratings: Vec<String>,
	pub adult_only: bool,
	pub author: Option<String>,
	pub years: (Option<f32>, Option<f32>),
	pub chapters: (Option<f32>, Option<f32>),
	pub official: bool,
}

impl Search {
	/// Settings first; filters then override them.
	pub fn new(query: Option<String>) -> Self {
		Self {
			query,
			excluded_genres: settings::hidden_genres(),
			excluded_tags: settings::hidden_tags(),
			types: settings::content_types(),
			ratings: settings::content_ratings(),
			adult_only: settings::adult_only(),
			..Default::default()
		}
	}

	/// The novel ranking, which the feeds can't serve whole.
	pub fn novels() -> Self {
		let mut search = Self::new(None);
		search.sort = POPULARITY;
		search.types.retain(|kind| kind == "Novel");
		search
	}

	pub fn matches_nothing(&self) -> bool {
		self.types.is_empty() || (self.ratings.is_empty() && !self.adult_only)
	}

	fn push_ids(clauses: &mut Vec<String>, field: &str, ids: &[String], any: bool) {
		if any && !ids.is_empty() {
			clauses.push(format!("{field}:={}", list(ids)));
		} else {
			clauses.extend(ids.iter().map(|id| format!("{field}:={}", quote(id))));
		}
	}

	fn type_clause(&self) -> Option<String> {
		let (comics, novels) = split_types(&self.types);
		if novels && comics.len() == CONTENT_TYPES.len() - 1 {
			return None;
		}
		// Some comics have no medium set, so comics are matched as "not a novel".
		let mut parts = Vec::new();
		if comics.len() == CONTENT_TYPES.len() - 1 {
			parts.push("medium:!=[`Novel`]".into());
		} else if !comics.is_empty() {
			parts.push(format!("(medium:!=[`Novel`] && type:={})", list(&comics)));
		}
		if novels {
			parts.push("medium:=[`Novel`]".into());
		}
		Some(format!("({})", parts.join(" || ")))
	}

	fn rating_clauses(&self, clauses: &mut Vec<String>) {
		if self.adult_only {
			clauses.push(format!(
				"(isAdult:=true || mbContentRating:=[`Pornographic`] || genreIds:=[{}])",
				quote(HENTAI)
			));
			return;
		}
		// Most novels are unrated, so unwanted ratings are excluded rather than the wanted
		// ones matched, and the genres stand in for the missing rating.
		let genres = excluded_rating_genres(&self.ratings);
		if !allows(&self.ratings, "Pornographic") {
			clauses.push("isAdult:=false".into());
		}
		let unwanted = RATINGS
			.into_iter()
			.filter(|rating| !allows(&self.ratings, rating))
			.collect::<Vec<_>>();
		if !unwanted.is_empty() {
			clauses.push(format!("mbContentRating:!={}", list(&unwanted)));
		}
		if !genres.is_empty() {
			clauses.push(format!("genreIds:!={}", list(&genres)));
		}
		// An unrated title without an adult genre counts as safe.
		if !allows(&self.ratings, "Safe") {
			clauses.push(format!(
				"(mbContentRating:=[`Suggestive`,`Erotica`,`Pornographic`] || genreIds:={} || isAdult:=true)",
				list(&[HENTAI, ADULT_GENRES[0], ADULT_GENRES[1]])
			));
		}
	}

	fn filter_by(&self) -> String {
		let mut clauses = Vec::new();
		Self::push_ids(&mut clauses, "genreIds", &self.genres, self.any_genre);
		if !self.excluded_genres.is_empty() {
			clauses.push(format!("genreIds:!={}", list(&self.excluded_genres)));
		}
		Self::push_ids(&mut clauses, "tagIds", &self.tags, self.any_tag);
		// A series has one demographic, so several can only mean any of them.
		Self::push_ids(&mut clauses, "tagIds", &self.demographics, true);
		if !self.excluded_tags.is_empty() {
			clauses.push(format!("tagIds:!={}", list(&self.excluded_tags)));
		}
		clauses.extend(self.type_clause());
		if !self.statuses.is_empty() {
			clauses.push(format!("status:={}", list(&self.statuses)));
		}
		if let Some(from) = self.years.0 {
			clauses.push(format!("releaseYear:>={}", from as i32));
		}
		if let Some(to) = self.years.1 {
			clauses.push(format!("releaseYear:<={}", to as i32));
		}
		if let Some(from) = self.chapters.0.filter(|from| *from > 0.0) {
			clauses.push(format!("chapterCount:>={}", from as i32));
		}
		if let Some(to) = self.chapters.1.filter(|to| *to > 0.0) {
			clauses.push(format!("chapterCount:<={}", to as i32));
		}
		if self.official {
			clauses.push("officialTranslation:=true".into());
		}
		if let Some(author) = &self.author {
			clauses.push(format!("authors:{}", quote(author)));
		}
		self.rating_clauses(&mut clauses);
		match self.sort {
			RELEVANCE | POPULARITY => clauses.push("views:>0".into()),
			TOP_RATED => clauses.push("mbRating:>0".into()),
			_ => {}
		}
		clauses.push("hidden:!=true".into());
		clauses.join(" && ")
	}

	pub fn url(&self, page: i32) -> String {
		let mut query = QueryParameters::new();
		query.push("q", Some(self.query.as_deref().unwrap_or("*")));
		query.push("query_by", Some(QUERY_BY));
		query.push("query_by_weights", Some(QUERY_BY_WEIGHTS));
		query.push("num_typos", Some(NUM_TYPOS));
		query.push("prefix", Some(PREFIX));
		query.push("include_fields", Some(CARD_FIELDS));
		query.push("filter_by", Some(&self.filter_by()));
		query.push("page", Some(&page.to_string()));
		query.push("per_page", Some(&SEARCH_LIMIT.to_string()));
		if self.query.is_some() {
			query.push("infix", Some(INFIX));
		}
		if self.query.is_none() || self.sort != RELEVANCE {
			let sort = SORTS.get(self.sort).copied().unwrap_or(SORTS[RELEVANCE]);
			query.push("sort_by", Some(sort));
		}
		format!("{SEARCH_URL}?{query}")
	}

	pub fn page(&self, page: i32, label_novels: bool) -> Result<MangaPageResult> {
		if self.matches_nothing() {
			return Ok(MangaPageResult::default());
		}
		let response: SearchResponse = get(&self.url(page))?;
		Ok(response.into_page(label_novels))
	}
}

fn api_url(path: &str, params: &[(&str, &str)]) -> String {
	let mut query = QueryParameters::new();
	for (name, value) in params {
		query.push(name, Some(value));
	}
	format!("{BASE_URL}/api/{path}?{query}")
}

pub fn manga_page_url(id: &str) -> String {
	api_url("manga/page", &[("id", id)])
}

pub fn chapters_url(id: &str) -> String {
	api_url("manga/allChapters", &[("mangaId", id)])
}

pub fn read_url(manga_id: &str, chapter_id: &str, novel: bool) -> String {
	let path = if novel {
		"read/novelChapter"
	} else {
		"read/chapter"
	};
	api_url(path, &[("mangaId", manga_id), ("chapterId", chapter_id)])
}
