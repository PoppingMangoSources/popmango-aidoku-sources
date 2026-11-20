use crate::{
	helpers::app_language,
	models::{ComicPage, Listing, Title},
	settings,
};
use aidoku::{
	ContentRating, Manga, MangaPageResult, Result,
	alloc::{String, Vec, format, string::ToString},
	helpers::uri::QueryParameters,
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;

pub const BASE_URL: &str = "https://swarm.ws";
pub const API_URL: &str = "https://swarm.ws/api";

const PAGE_SIZE: usize = 50;

// Sorts in `filters.json` order.
// Browse answers `created` with an error; `newest` is the same sort.
pub const SORTS: [&str; 3] = ["last_updated", "title", "newest"];

#[derive(Default)]
pub struct Content {
	pub rating: String,
	pub types: Option<Vec<String>>,
	pub excluded_genres: Vec<String>,
	pub excluded_tags: Vec<String>,
	pub languages: Vec<String>,
}

impl Content {
	pub fn new() -> Self {
		Self {
			rating: settings::content_rating(),
			types: settings::types(),
			excluded_genres: settings::excluded_genres(),
			excluded_tags: settings::excluded_tags(),
			languages: settings::languages(),
		}
	}

	// The site's rating parameters, as its own pages send them.
	pub fn rating_query(&self, query: &mut QueryParameters, feed: bool) {
		match self.rating.as_str() {
			"safe" => {
				query.push_encoded("nsfw", Some("false"));
				query.push_encoded("suggestive", Some("false"));
			}
			"nsfw" => {
				query.push_encoded("nsfw", Some("true"));
				if feed {
					query.push_encoded("nsfwOnly", Some("true"));
					query.push_encoded("suggestive", Some("true"));
				}
			}
			_ if feed => {
				query.push_encoded("nsfw", Some("true"));
				query.push_encoded("suggestive", Some("true"));
			}
			_ => {}
		}
	}

	// A cleared language or type selection shows nothing.
	pub fn cleared(&self) -> bool {
		self.languages.is_empty() || self.types.as_ref().is_some_and(|types| types.is_empty())
	}

	pub fn allows_novels(&self) -> bool {
		self.languages.iter().any(|language| language == "en")
			&& self
				.types
				.as_ref()
				.is_none_or(|types| types.iter().any(|kind| kind == "Novel"))
	}

	pub fn allows_rating(&self, rating: ContentRating) -> bool {
		match self.rating.as_str() {
			"safe" => rating == ContentRating::Safe,
			"nsfw" => rating == ContentRating::NSFW,
			_ => true,
		}
	}

	// Applied to whatever each record carries; records without a field pass it.
	pub fn allows(&self, title: &Title) -> bool {
		let kind = title.kind.as_deref().filter(|kind| !kind.is_empty());
		let typed = self.types.as_ref().is_none_or(|types| {
			kind.is_none_or(|kind| types.iter().any(|t| t.eq_ignore_ascii_case(kind)))
		});
		let translated = title.languages.is_empty()
			|| title
				.languages
				.iter()
				.any(|code| self.languages.iter().any(|l| l == app_language(code)));
		let excluded_genre = title
			.genres
			.iter()
			.any(|genre| self.excluded_genres.contains(genre));
		let excluded_tag = title
			.tags
			.iter()
			.any(|tag| self.excluded_tags.contains(&tag.to_lowercase()));
		self.allows_rating(title.content_rating())
			&& typed && translated
			&& !excluded_genre
			&& !excluded_tag
	}
}

#[derive(Default)]
pub struct SearchParams {
	pub query: Option<String>,
	pub sort: &'static str,
	pub genres: Vec<String>,
	pub tags: Vec<String>,
	pub chapters: (Option<f32>, Option<f32>),
	pub years: (Option<f32>, Option<f32>),
	pub content: Content,
}

impl SearchParams {
	pub fn new(query: Option<String>) -> Self {
		Self {
			query,
			sort: SORTS[0],
			content: Content::new(),
			..Default::default()
		}
	}

	pub fn page(&self, page: i32) -> Result<MangaPageResult> {
		if self.content.cleared() {
			return Ok(MangaPageResult::default());
		}
		let offset = (page as usize - 1) * PAGE_SIZE;
		let mut query = QueryParameters::new();
		if self.query.is_some() {
			query.push("title", self.query.as_deref());
		}
		query.push_encoded("limit", Some(&PAGE_SIZE.to_string()));
		query.push_encoded("offset", Some(&offset.to_string()));
		query.push_encoded("order", Some(self.sort));
		if !self.genres.is_empty() {
			query.push("genres", Some(&self.genres.join(",")));
		}
		// Titles carry their type in either case.
		if let Some(types) = &self.content.types {
			let both: Vec<String> = types
				.iter()
				.flat_map(|kind| [kind.clone(), kind.to_lowercase()])
				.collect();
			query.push("types", Some(&both.join(",")));
		}
		if !self.tags.is_empty() {
			query.push("tags", Some(&self.tags.join(",")));
		}
		self.content.rating_query(&mut query, false);
		let number = |value: f32| (value as i32).to_string();
		if let Some(from) = self.years.0 {
			query.push_encoded("yearStartMin", Some(&number(from)));
		}
		if let Some(to) = self.years.1 {
			query.push_encoded("yearStartMax", Some(&number(to)));
		}
		if let Some(from) = self.chapters.0.filter(|from| *from > 0.0) {
			query.push_encoded("minChapters", Some(&number(from)));
		}
		let path = if self.query.is_some() {
			"comic/search"
		} else {
			"manga/filtered"
		};
		// Both paths are fixed, so the query starts here.
		let response: Listing<Title> = get(&format!("{API_URL}/{path}?{query}"))?;
		// With a filter set, the total is only the offset plus one page.
		let has_next_page = !response.data.is_empty()
			&& response.total.unwrap_or_default() > (offset + response.data.len()) as i64;
		// The site takes no maximum; text search results carry no count.
		let fits = |title: &Title| match (self.chapters.1, title.chapter_count) {
			(Some(to), Some(count)) => count <= to,
			_ => true,
		};
		Ok(MangaPageResult {
			entries: response
				.data
				.into_iter()
				.filter(|title| self.content.allows(title) && fits(title))
				.filter_map(|title| Manga::try_from(title).ok())
				.collect(),
			has_next_page,
		})
	}
}

// The comic page carries the series and every upload as `initialData`.
pub fn comic_page(slug: &str) -> Result<ComicPage> {
	// A series key is a bare slug, so the query starts here.
	let response = Request::get(format!("{BASE_URL}/comic/{slug}?_rsc"))?
		.header("RSC", "1")
		.send()?;
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("Swarm returned status {status}");
	}
	let body = response.get_string()?;
	// The first `initialData` that reads as a comic page; another object can carry the name.
	let marker = r#""initialData":"#;
	body.match_indices(marker)
		.find_map(|(start, _)| {
			serde_json::Deserializer::from_str(&body[start + marker.len()..])
				.into_iter::<ComicPage>()
				.next()?
				.ok()
		})
		.ok_or_else(|| error!("Series not found"))
}

pub fn site_headers(request: Request) -> Request {
	request
		.header("Accept", "application/json")
		.header("Referer", &format!("{BASE_URL}/"))
}

pub fn request(url: &str) -> Result<Request> {
	Ok(site_headers(Request::get(url)?))
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("Swarm returned status {status}");
	}
	response.get_json_owned()
}

pub fn get<T: DeserializeOwned>(url: &str) -> Result<T> {
	parse(request(url)?.send()?)
}
