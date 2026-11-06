use crate::{
	API_URL, PAGE_SIZE,
	models::{ErrorResponse, NovelList},
};
use aidoku::{
	ContentRating, MangaPageResult, Result,
	alloc::{String, Vec, string::ToString},
	helpers::uri::QueryParameters,
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;

/// The genres the site's library leaves out when hiding adult novels, sent as exclusions like it does.
pub const ADULT_GENRES: [&str; 12] = [
	"adult",
	"smut",
	"mature",
	"erotica",
	"ecchi",
	"hentai",
	"explicit",
	"sexual content",
	"nsfw",
	"r-18",
	"lewd",
	"pornographic",
];

/// Values the genre list carries that the site treats as categories, not genres.
pub const NON_GENRES: [&str; 7] = [
	"browse",
	"completed novel",
	"completed novels",
	"latest novels",
	"latest novel",
	"anime & comics",
	"anime and comics",
];

pub fn get<T: DeserializeOwned>(url: &str) -> Result<T> {
	parse(Request::get(url)?.send()?)
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		match response.get_json_owned::<ErrorResponse>() {
			Ok(body) => bail!("Novel Archive: {}", body.error),
			Err(_) => bail!("Novel Archive returned status {status}"),
		}
	}
	response.get_json_owned()
}

#[derive(Default)]
pub struct SearchParams {
	pub query: Option<String>,
	pub sort: Option<&'static str>,
	pub status: Option<String>,
	pub any_genre: bool,
	pub genres: Vec<String>,
	pub hidden_genres: Vec<String>,
	pub hide_adult: bool,
}

impl SearchParams {
	pub fn url(mut self, page: i32) -> String {
		if self.hide_adult {
			self.hidden_genres
				.extend(ADULT_GENRES.iter().map(|genre| genre.to_string()));
		}
		let mut qs = QueryParameters::new();
		qs.push("page", Some(&page.to_string()));
		qs.push("per_page", Some(&PAGE_SIZE.to_string()));
		if let Some(query) = &self.query {
			qs.push("search", Some(query));
			qs.push("fuzzy", Some("1"));
		}
		if let Some(sort) = self.sort {
			qs.push("sort", Some(sort));
		}
		if let Some(status) = &self.status {
			qs.push("status", Some(status));
		}
		if !self.genres.is_empty() {
			qs.push("genres_include", Some(&self.genres.join(",")));
		}
		if !self.hidden_genres.is_empty() {
			qs.push("genres_exclude", Some(&self.hidden_genres.join(",")));
		}
		if self.any_genre {
			qs.push("genre_match", Some("any"));
		}
		format!("{API_URL}?{qs}")
	}
}

/// One page of the library's browse, the endpoint behind search, listings and sorted rows.
pub fn browse(params: SearchParams, page: i32) -> Result<MangaPageResult> {
	let list: NovelList = get(&params.url(page))?;
	Ok(MangaPageResult {
		has_next_page: list
			.pagination
			.is_some_and(|pagination| pagination.has_next),
		entries: list.novels.into_iter().map(Into::into).collect(),
	})
}

/// A novel's genres as the site lists them: deduplicated, without its category values.
pub fn genres(list: &str) -> Vec<String> {
	let mut genres: Vec<String> = Vec::new();
	for genre in list.split(',').map(str::trim) {
		let lowered = genre.to_lowercase();
		if genre.is_empty()
			|| NON_GENRES.contains(&lowered.as_str())
			|| genres.iter().any(|seen| seen.to_lowercase() == lowered)
		{
			continue;
		}
		genres.push(genre.into());
	}
	genres
}

pub fn content_rating(list: &str) -> ContentRating {
	let mut rating = ContentRating::Safe;
	for genre in list.split(',').map(|genre| genre.trim().to_lowercase()) {
		match genre.as_str() {
			"mature" | "ecchi" => rating = ContentRating::Suggestive,
			genre if ADULT_GENRES.contains(&genre) => return ContentRating::NSFW,
			_ => {}
		}
	}
	rating
}

/// The chapter's name without a leading "Chapter n", which the app already shows.
pub fn chapter_title(name: &str, number: usize) -> Option<String> {
	let name = name.trim();
	let prefix = format!("chapter {number}");
	let rest = name
		.get(..prefix.len())
		.filter(|start| start.eq_ignore_ascii_case(&prefix))
		.map(|_| &name[prefix.len()..])
		.filter(|rest| !rest.starts_with(|c: char| c.is_ascii_digit()))
		.map(|rest| {
			rest.trim_start_matches([' ', ':', '-', '\u{2013}', '\u{2014}', '.'])
				.trim()
		})
		.unwrap_or(name);
	(!rest.is_empty()).then(|| rest.into())
}
