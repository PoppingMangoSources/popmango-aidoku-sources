use crate::{
	models::{BrowsePage, ErrorResponse, Rpc, Terms, Years, dedup_keep_order},
	settings::{content_ratings, excluded_genres, languages, origins},
};
use aidoku::{
	Manga, MangaPageResult, Result,
	alloc::{String, Vec, format},
	imports::net::{Request, Response},
	prelude::*,
};
use serde::{Serialize, de::DeserializeOwned};

pub const BASE_URL: &str = "https://onisaga.com";
pub const CDN_URL: &str = "https://media.onisaga.com";
pub const CHAPTER_PAGE_SIZE: i32 = 200;

// Sorts in `filters.json` order; `""` resolves to relevance with a query or latest update without.
pub const SORTS: [&str; 5] = ["", "updated", "new", "title", "published"];

const PAGE_SIZE: i32 = 42;
const MAX_LANGUAGES: usize = 10;

#[derive(Serialize)]
pub struct SearchParams {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub q: Option<String>,
	pub medium: [&'static str; 1],
	#[serde(skip_serializing_if = "Option::is_none")]
	pub origin: Option<Vec<String>>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub form: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub status: Vec<String>,
	#[serde(skip_serializing_if = "Vec::is_empty")]
	pub demographic: Vec<String>,
	pub ratings: Vec<String>,
	pub languages: Vec<String>,
	#[serde(skip_serializing_if = "Terms::is_empty")]
	pub genres: Terms,
	#[serde(skip_serializing_if = "Terms::is_empty")]
	pub themes: Terms,
	#[serde(skip_serializing_if = "Terms::is_empty")]
	pub tags: Terms,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub year: Option<Years>,
	#[serde(skip_serializing_if = "core::ops::Not::not")]
	pub readable: bool,
	pub sort: &'static str,
	pub limit: i32,
	#[serde(skip_serializing_if = "Option::is_none")]
	pub cursor: Option<String>,
}

impl SearchParams {
	pub fn new(q: Option<String>) -> Self {
		let mut languages = languages();
		dedup_keep_order(&mut languages);
		languages.truncate(MAX_LANGUAGES);
		let mut genres = Terms {
			include: Vec::new(),
			exclude: excluded_genres(),
		};
		genres.finalize();
		Self {
			sort: if q.is_some() { "relevance" } else { "updated" },
			q,
			medium: ["comic"],
			origin: origins(),
			form: Vec::new(),
			status: Vec::new(),
			demographic: Vec::new(),
			ratings: content_ratings(),
			languages,
			genres,
			themes: Terms::default(),
			tags: Terms::default(),
			year: None,
			readable: false,
			limit: PAGE_SIZE,
			cursor: None,
		}
	}

	pub fn sorted(sort: &'static str) -> Self {
		Self {
			sort,
			..Self::new(None)
		}
	}

	pub fn finalize(&mut self) {
		if let Some(origin) = &mut self.origin {
			dedup_keep_order(origin);
		}
		dedup_keep_order(&mut self.form);
		dedup_keep_order(&mut self.status);
		dedup_keep_order(&mut self.demographic);
		dedup_keep_order(&mut self.ratings);
		dedup_keep_order(&mut self.languages);
		self.languages.truncate(MAX_LANGUAGES);
		self.genres.finalize();
		self.themes.finalize();
		self.tags.finalize();
		if self.sort.is_empty() || (self.sort == "relevance" && self.q.is_none()) {
			self.sort = if self.q.is_some() {
				"relevance"
			} else {
				"updated"
			};
		}
	}

	pub fn is_empty(&self) -> bool {
		self.ratings.is_empty()
			|| self.languages.is_empty()
			|| self.origin.as_ref().is_some_and(Vec::is_empty)
	}

	pub fn adult(&self) -> bool {
		self.ratings
			.iter()
			.any(|rating| rating == "mature" || rating == "explicit")
	}

	// `catalog/home` cards lack genre lists and default to English-heavy shelves.
	pub fn shelves_narrowed(&self) -> bool {
		!self.genres.exclude.is_empty()
			|| self.origin.is_some()
			|| !self.languages.iter().any(|lang| lang == "en")
	}

	pub fn key(&self) -> Result<String> {
		Ok(serde_json::to_string(self)?)
	}

	pub fn request(&self) -> Result<Request> {
		rpc("catalog/browse", self)
	}

	pub fn page(&self) -> Result<(MangaPageResult, Option<String>)> {
		if self.is_empty() {
			return Ok((MangaPageResult::default(), None));
		}
		let page = parse::<BrowsePage>(self.request()?.send()?)?;
		Ok((
			MangaPageResult {
				entries: page
					.items
					.into_iter()
					.filter_map(|work| Manga::try_from(work).ok())
					.collect(),
				has_next_page: page.next_cursor.is_some(),
			},
			page.next_cursor,
		))
	}
}

pub fn rpc<T: Serialize>(method: &str, input: T) -> Result<Request> {
	let body = serde_json::to_vec(&Rpc { json: input })?;
	Ok(Request::post(format!("{BASE_URL}/api/rpc/{method}"))?
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.header("Origin", BASE_URL)
		.header("Referer", &format!("{BASE_URL}/"))
		.body(body))
}

pub fn set_age(adult: bool) -> Result<()> {
	let body = if adult {
		"action=confirm&next=%2Frobots.txt"
	} else {
		"action=withdraw&next=%2Frobots.txt"
	};
	Request::post(format!("{BASE_URL}/api/age-confirmation"))?
		.header("Content-Type", "application/x-www-form-urlencoded")
		.header("Origin", BASE_URL)
		.header("Referer", &format!("{BASE_URL}/"))
		.body(body)
		.send()?;
	Ok(())
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		if status == 403
			&& response
				.get_header("cf-mitigated")
				.is_some_and(|value| value == "challenge")
		{
			bail!("OniSaga blocked the request behind a Cloudflare check");
		}
		if let Ok(err) = response.get_json_owned::<ErrorResponse>()
			&& let Some(message) = err.error.message
			&& !message.is_empty()
		{
			bail!("{message}");
		}
		bail!("OniSaga returned HTTP {status}");
	}
	Ok(response.get_json_owned::<Rpc<T>>()?.json)
}
