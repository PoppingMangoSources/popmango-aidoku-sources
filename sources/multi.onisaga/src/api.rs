use crate::{
	models::{BrowsePage, Rpc, Terms, Years},
	settings,
};
use aidoku::{
	Manga, MangaPageResult, Result,
	alloc::{String, Vec},
	imports::{
		html::Document,
		net::{Request, Response},
	},
	prelude::*,
};
use serde::{Serialize, de::DeserializeOwned};

pub const BASE_URL: &str = "https://onisaga.com";
pub const CDN_URL: &str = "https://media.onisaga.com";
pub const PAGE_SIZE: i32 = 60;

// Sorts in `filters.json` order.
pub const SORTS: [&str; 4] = ["updated", "new", "title", "published"];

#[derive(Default, Serialize)]
pub struct SearchParams {
	#[serde(skip_serializing_if = "Option::is_none")]
	pub q: Option<String>,
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
		Self {
			// With a query the site orders by relevance unless asked otherwise.
			sort: if q.is_some() { "relevance" } else { SORTS[0] },
			q,
			origin: settings::origins(),
			ratings: settings::content_ratings(),
			languages: settings::languages(),
			genres: Terms {
				exclude: settings::excluded_genres(),
				..Default::default()
			},
			limit: PAGE_SIZE,
			..Default::default()
		}
	}

	pub fn sorted(sort: &'static str) -> Self {
		Self {
			sort,
			..Self::new(None)
		}
	}

	// A cleared setting browses nothing.
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
		.body(body))
}

// Guests see Mature and Explicit titles only after stating they are 18 or older.
pub fn confirm_age() -> Result<()> {
	Request::post(format!("{BASE_URL}/api/age-confirmation"))?
		.header("Content-Type", "application/x-www-form-urlencoded")
		.header("Origin", BASE_URL)
		.header("Referer", &format!("{BASE_URL}/"))
		.body("action=confirm&next=%2Frobots.txt")
		.send()?;
	Ok(())
}

fn checked(response: Response) -> Result<Response> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("OniSaga returned status {status}");
	}
	Ok(response)
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	Ok(checked(response)?.get_json_owned::<Rpc<T>>()?.json)
}

pub fn html(response: Response) -> Result<Document> {
	Ok(checked(response)?.get_html()?)
}

pub fn get(url: &str) -> Result<Document> {
	html(Request::get(url)?.send()?)
}
