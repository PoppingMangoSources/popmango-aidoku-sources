use crate::{
	helpers::{ADULT_GENRES, HENTAI, allows, spotlight_genre, take_document},
	models::{ErrorResponse, FeedResponse, SearchResponse},
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
pub const API_URL: &str = "https://atsu.moe/api";

const QUERY_BY: &str = "title,englishTitle,otherNames,authors,acronyms";
const QUERY_BY_WEIGHTS: &str = "4,3,2,2,1";
const NUM_TYPOS: &str = "4,3,2,1,0";
const PREFIX: &str = "true,true,true,true,false";
const INFIX: &str = "off,off,fallback,off,off";
pub const CARD_FIELDS: &str =
	"id,title,poster,posterMedium,isAdult,mbContentRating,genreIds,medium,status";
pub const RATING_FIELDS: &str = "id,isAdult,mbContentRating,genreIds";
pub const FEATURED_FIELDS: &str =
	"id,synopsis,authors,status,tags,isAdult,mbContentRating,genreIds";

/// Indexed sorts in `filters.json` order, then the ranked feeds, then Random.
const SORTS: [&str; 6] = [
	"views",
	"title",
	"trending",
	"dateAdded",
	"releaseDate",
	"mbRating",
];
const FEEDS: [&str; 9] = [
	"hotUpdates",
	"recentlyUpdated",
	"rising",
	"hotArrivals",
	"mostBookmarked",
	"mostTalkedAbout",
	"bingeWorthy",
	"mostPolarizing",
	"hiddenGems",
];
/// The feeds the site ranks over a period.
pub const RANKED: [&str; 3] = ["popular", "mostBookmarked", "mostTalkedAbout"];
const RANDOM: usize = SORTS.len() + FEEDS.len();
const POPULARITY: usize = 0;
const TOP_RATED: usize = 5;

pub fn request(url: &str) -> Result<Request> {
	Ok(Request::get(url)?
		.header("Accept", "application/json")
		.header("Referer", &format!("{BASE_URL}/")))
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		match response.get_json_owned::<ErrorResponse>() {
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

pub fn lookup_url(ids: &[&str], fields: &str) -> String {
	let mut query = QueryParameters::new();
	query.push("q", Some("*"));
	query.push("query_by", Some("title"));
	query.push("include_fields", Some(fields));
	query.push("filter_by", Some(&format!("id:={}", list(ids))));
	query.push("per_page", Some(&ids.len().max(1).to_string()));
	format!("{BASE_URL}/collections/manga/documents/search?{query}")
}

#[derive(Default)]
pub struct SearchParams {
	pub query: Option<String>,
	pub sort: usize,
	pub ascending: bool,
	pub seed: u32,
	pub genres: Vec<String>,
	pub excluded_genres: Vec<String>,
	pub any_genre: bool,
	pub tags: Vec<String>,
	pub excluded_tags: Vec<String>,
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

impl SearchParams {
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

	// The novel feed is incomplete; use indexed popularity.
	pub fn novels() -> Self {
		let mut search = Self::new(None);
		search.types.retain(|kind| kind == "Novel");
		search
	}

	pub fn feed_url(&self, id: &str, page: i32, timeframe: Option<&str>) -> Result<String> {
		let mut query = QueryParameters::new();
		match id {
			"genreSpotlight" => query.push("genre", Some(spotlight_genre())),
			"popular" | "mostBookmarked" | "mostTalkedAbout" | "hotUpdates" | "recentlyUpdated"
			| "rising" | "hotArrivals" | "recentlyAdded" | "topRated" | "hiddenGems"
			| "bingeWorthy" | "mostPolarizing" => {}
			_ => bail!("Unknown listing"),
		}
		if let Some(timeframe) = timeframe {
			query.push("timeframe", Some(timeframe));
		}
		let (comics, novels) = split_types(&self.types);
		query.push("offset", Some(&((page - 1) * FEED_LIMIT).to_string()));
		query.push("limit", Some(&FEED_LIMIT.to_string()));
		// A full selection is left out, as in search. The feeds AND types with mediums, so with
		// novels on, types are filtered locally.
		if comics.is_empty() {
			query.push("mediums", Some("Novel"));
		} else if !novels {
			query.push("mediums", Some("Comic"));
			if comics.len() < CONTENT_TYPES.len() - 1 {
				query.push("types", Some(&comics.join(",")));
			}
		}
		let mut excluded = self
			.excluded_genres
			.iter()
			.chain(&self.excluded_tags)
			.map(String::as_str)
			.collect::<Vec<_>>();
		if self.adult_only {
			query.push("adult", Some("1"));
		} else {
			// The feeds take genre and tag ids in one parameter; the two never share an id.
			excluded.extend(excluded_rating_genres(&self.ratings));
			// Left out, it hides adult titles, so even every rating is sent.
			query.push("contentRatings", Some(&self.ratings.join(",")));
		}
		if !excluded.is_empty() {
			query.push("excludedTags", Some(&excluded.join(",")));
		}
		Ok(format!("{API_URL}/home2/{id}?{query}"))
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
		// Most novels are unrated, so exclude unwanted ratings rather than match wanted ones.
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
		Self::push_ids(&mut clauses, "tagIds", &self.tags, false);
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
			POPULARITY => clauses.push("views:>0".into()),
			TOP_RATED => clauses.push("mbRating:>0".into()),
			_ => {}
		}
		clauses.push("hidden:!=true".into());
		clauses.join(" && ")
	}

	pub fn url(&self, page: i32, ids: &[&str]) -> String {
		let mut query = QueryParameters::new();
		query.push("q", Some(self.query.as_deref().unwrap_or("*")));
		query.push("query_by", Some(QUERY_BY));
		query.push("query_by_weights", Some(QUERY_BY_WEIGHTS));
		query.push("num_typos", Some(NUM_TYPOS));
		query.push("prefix", Some(PREFIX));
		query.push("include_fields", Some(CARD_FIELDS));
		let mut filter = self.filter_by();
		if !ids.is_empty() {
			filter.push_str(&format!(" && id:={}", list(ids)));
		}
		query.push("filter_by", Some(&filter));
		query.push("page", Some(&page.to_string()));
		query.push("per_page", Some(&SEARCH_LIMIT.to_string()));
		if self.query.is_some() {
			query.push("infix", Some(INFIX));
		}
		let sort = if self.sort == RANDOM {
			let fields = ["shuffleA", "shuffleB"];
			let seed = self.seed.max(1);
			let first = fields[seed as usize % 2];
			let second = fields[(seed as usize + 1) % 2];
			if seed >> 1 & 1 == 1 {
				format!("_eval({first}:<={seed}):desc,{first}:desc,{second}:desc")
			} else {
				format!("_eval({first}:>={seed}):desc,{first}:asc,{second}:asc")
			}
		} else {
			// A feed sort only uses this for its lookup, whose order is discarded.
			let field = SORTS.get(self.sort).copied().unwrap_or(SORTS[POPULARITY]);
			format!("{field}:{}", if self.ascending { "asc" } else { "desc" })
		};
		query.push("sort_by", Some(&sort));
		format!("{BASE_URL}/collections/manga/documents/search?{query}")
	}

	pub fn page(&self, page: i32, label_novels: bool) -> Result<MangaPageResult> {
		if self.matches_nothing() {
			return Ok(MangaPageResult::default());
		}
		let feed = match self.sort.checked_sub(SORTS.len()) {
			Some(index) => FEEDS.get(index).map(|id| {
				let timeframe = RANKED.contains(id).then(|| settings::timeframe(id));
				(*id, timeframe)
			}),
			// Popularity over a period is the popular feed; over all time, the index's views.
			None if self.sort == POPULARITY => Some(settings::timeframe("popular"))
				.filter(|timeframe| timeframe != "all")
				.map(|timeframe| ("popular", Some(timeframe))),
			None => None,
		};
		if let Some((id, timeframe)) = feed {
			let response: FeedResponse = get(&self.feed_url(id, page, timeframe.as_deref())?)?;
			let has_next_page = response.items.len() as i32 == FEED_LIMIT;
			if response.items.is_empty() {
				return Ok(MangaPageResult::default());
			}
			// One indexed lookup applies every search filter while preserving feed order.
			let ids = response
				.items
				.iter()
				.map(|item| item.id.as_str())
				.collect::<Vec<_>>();
			let matched: SearchResponse = get(&self.url(1, &ids))?;
			let mut documents = matched.hits.into_iter().map(|hit| hit.document).collect();
			let entries = response
				.items
				.into_iter()
				.filter_map(|item| {
					take_document(&mut documents, &item.id)
						.map(|document| document.into_manga(label_novels))
				})
				.collect();
			return Ok(MangaPageResult {
				entries,
				has_next_page,
			});
		}
		let response: SearchResponse = get(&self.url(page, &[]))?;
		Ok(response.into_page(label_novels))
	}
}

fn api_url(path: &str, params: &[(&str, &str)]) -> String {
	let mut query = QueryParameters::new();
	for (name, value) in params {
		query.push(name, Some(value));
	}
	format!("{API_URL}/{path}?{query}")
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
