use aidoku::{
	ContentRating, MangaStatus, Result, Viewer,
	alloc::{String, Vec, format, vec},
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;
use serde_json::{Map, Value, json};

use crate::{
	models::{SourceList, UploadSource},
	settings,
};

pub const PAGE_SIZE: i32 = 35;

/// The site rejects a list of more ids than this.
const MAX_IDS: usize = 50;

/// Sort fields in `res/filters.json` order.
pub const SORTS: &[&str] = &[
	"",
	"total_views",
	"avg_views",
	"avg_views_today",
	"avg_views_week",
	"avg_views_month",
	"updated_at",
	"current_books",
	"created_at",
	"popular_now",
	"rating",
];

pub fn api_url(base_url: &str, path: &str) -> String {
	format!("{base_url}/api/v2{path}")
}

pub fn site_request(request: Request, base_url: &str) -> Request {
	request
		.header("Accept", "application/json")
		.header("Origin", base_url)
		.header("Referer", &format!("{base_url}/"))
}

pub fn get(base_url: &str, url: &str) -> Result<Request> {
	Ok(site_request(Request::get(url)?, base_url))
}

pub fn post(base_url: &str, url: &str, body: &Value) -> Result<Request> {
	Ok(site_request(Request::post(url)?, base_url)
		.header("Content-Type", "application/json")
		.body(serde_json::to_string(body).unwrap_or_default()))
}

pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("Kagane returned status {status}");
	}
	response.get_json_owned()
}

pub fn sources_request(base_url: &str) -> Result<Request> {
	post(
		base_url,
		&api_url(base_url, "/sources/list"),
		&json!({ "source_types": null }),
	)
}

pub fn sources(response: Response) -> Vec<UploadSource> {
	parse::<SourceList>(response)
		.map(|list| list.sources)
		.unwrap_or_default()
}

/// Sends `request`, with the upload sources alongside when titles name them.
pub fn send_with_sources(
	base_url: &str,
	request: Request,
) -> Result<(Response, Vec<UploadSource>)> {
	if !settings::show_source() {
		return Ok((request.send()?, Vec::new()));
	}
	let mut responses = Request::send_all(vec![request, sources_request(base_url)?]).into_iter();
	let response = responses
		.next()
		.ok_or_else(|| error!("Kagane sent no response"))??;
	let sources = responses
		.next()
		.and_then(|response| response.ok())
		.map(sources)
		.unwrap_or_default();
	Ok((response, sources))
}

/// The name a series' chapters are credited to, ticked when the source is official.
pub fn uploader(sources: &[UploadSource], id: Option<&str>) -> Option<String> {
	let source = sources
		.iter()
		.find(|source| Some(source.source_id.as_str()) == id)?;
	Some(if source.source_type.as_deref() == Some("Official") {
		format!("{} ✓", source.title)
	} else {
		source.title.clone()
	})
}

pub fn format_title(title: &str, source: Option<&str>, edition: Option<&str>) -> String {
	let mut title = String::from(title.trim());
	if let Some(edition) = edition.map(str::trim).filter(|edition| !edition.is_empty()) {
		title.push_str(&format!(" ({edition})"));
	}
	if let Some(source) = source {
		title.push_str(&format!(" [{source}]"));
	}
	title
}

/// The site rates by age band, with a flag on the pornographic titles in the 18+ band.
pub fn content_rating(age: Option<&str>, pornographic: Option<bool>) -> ContentRating {
	match age {
		_ if pornographic == Some(true) => ContentRating::NSFW,
		Some("Safe") => ContentRating::Safe,
		Some("FifteenPlus") => ContentRating::Suggestive,
		Some("EighteenPlus") => ContentRating::NSFW,
		_ => ContentRating::Unknown,
	}
}

/// The age bands for the ratings offered, and the flag that tells Erotica from Pornographic
/// within the 18+ band.
fn rating_query(ratings: &[String]) -> (Vec<&'static str>, Option<bool>) {
	let mut bands = Vec::new();
	for rating in ratings {
		let band = match rating.as_str() {
			"Safe" => "Safe",
			"Suggestive" => "FifteenPlus",
			_ => "EighteenPlus",
		};
		if !bands.contains(&band) {
			bands.push(band);
		}
	}
	let pornographic = if !ratings.iter().any(|rating| rating == "Pornographic") {
		Some(false)
	} else if ratings.len() == 1 {
		Some(true)
	} else {
		None
	};
	(bands, pornographic)
}

pub fn parse_status(s: &str) -> MangaStatus {
	match s.to_uppercase().as_str() {
		"ONGOING" => MangaStatus::Ongoing,
		"COMPLETED" => MangaStatus::Completed,
		"HIATUS" => MangaStatus::Hiatus,
		"ABANDONED" => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

pub fn parse_viewer(format: Option<&str>) -> Viewer {
	match format {
		Some("Manga") => Viewer::RightToLeft,
		Some("Comic") => Viewer::LeftToRight,
		_ => Viewer::Webtoon,
	}
}

fn checked(values: Vec<String>, label: &str) -> Result<Vec<String>> {
	let mut unique: Vec<String> = Vec::new();
	for value in values {
		let value = value.to_lowercase();
		if !unique.contains(&value) {
			unique.push(value);
		}
	}
	if unique.len() > MAX_IDS {
		bail!(
			"Kagane allows at most {MAX_IDS} {label}, and {} are selected",
			unique.len()
		);
	}
	Ok(unique)
}

#[derive(Default)]
pub struct Selection {
	pub included: Vec<String>,
	pub excluded: Vec<String>,
	pub match_any: bool,
}

impl Selection {
	/// A pick overrides the hide-list, since an id sent both ways matches nothing.
	fn into_value(self, hidden: Vec<String>, label: &str) -> Result<Option<Value>> {
		let values = checked(self.included, &format!("included {label}"))?;
		let excluded = checked(
			self.excluded
				.into_iter()
				.chain(hidden)
				.filter(|id| !values.contains(&id.to_lowercase()))
				.collect(),
			&format!("excluded {label}"),
		)?;
		if values.is_empty() && excluded.is_empty() {
			return Ok(None);
		}
		let mut object = Map::new();
		if !values.is_empty() {
			object.insert("match_all".into(), json!(!self.match_any));
		}
		object.insert("values".into(), json!(values));
		if !excluded.is_empty() {
			object.insert("exclude".into(), json!(excluded));
		}
		Ok(Some(Value::Object(object)))
	}
}

#[derive(Default)]
pub struct SearchParams {
	pub query: String,
	pub sort: &'static str,
	pub ascending: bool,
	pub exact_match: bool,
	/// Overrides the content rating setting when not empty.
	pub ratings: Vec<String>,
	pub formats: Vec<String>,
	pub upload_statuses: Vec<String>,
	pub publication_statuses: Vec<String>,
	pub original_languages: Vec<String>,
	pub sources: Vec<String>,
	pub genres: Selection,
	pub tags: Selection,
	pub disturbing: Option<bool>,
	pub staff: Option<String>,
	pub group: Option<String>,
	/// API field, lower bound and upper bound.
	pub ranges: Vec<(String, Option<f32>, Option<f32>)>,
	/// Resolved ids of the hidden tags setting.
	pub hidden_tags: Vec<String>,
}

impl SearchParams {
	pub fn sorted(sort: &'static str) -> Self {
		Self {
			sort,
			..Default::default()
		}
	}

	/// The search request, or `None` when the settings leave nothing to match.
	pub fn request(self, base_url: &str, page: i32, size: i32) -> Result<Option<Request>> {
		let ratings = if self.ratings.is_empty() {
			settings::content_ratings()
		} else {
			self.ratings
		};
		let source_types = settings::source_types();
		if ratings.is_empty() || source_types.as_ref().is_some_and(Vec::is_empty) {
			return Ok(None);
		}

		let mut body = Map::new();
		// The hide-lists keep browsing clear; a title search asks for that title regardless.
		let browsing = self.query.is_empty();
		if !browsing {
			body.insert("title".into(), json!(self.query));
		}
		body.insert(
			"content_lang".into(),
			json!(checked(settings::languages(), "languages")?),
		);
		if let Some(types) = source_types {
			body.insert("source_type".into(), json!(types));
		}
		let (bands, pornographic) = rating_query(&ratings);
		body.insert("age_rating".into(), json!(bands));
		if let Some(pornographic) = pornographic {
			body.insert("is_pornographic".into(), json!(pornographic));
		}
		if let Some(disturbing) = self.disturbing {
			body.insert("is_disturbing".into(), json!(disturbing));
		}
		for (field, values) in [
			("format", self.formats),
			("upload_status", self.upload_statuses),
			("publication_status", self.publication_statuses),
		] {
			if !values.is_empty() {
				body.insert(field.into(), json!(values));
			}
		}
		if !self.original_languages.is_empty() {
			body.insert(
				"original_language".into(),
				json!(checked(self.original_languages, "original languages")?),
			);
		}
		if !self.sources.is_empty() {
			body.insert("source_id".into(), json!(checked(self.sources, "sources")?));
		}

		let (hidden_genres, hidden_tags) = if browsing {
			(settings::hidden_genres(), self.hidden_tags)
		} else {
			(Vec::new(), Vec::new())
		};
		if let Some(genres) = self.genres.into_value(hidden_genres, "genres")? {
			body.insert("genres".into(), genres);
		}
		if let Some(tags) = self.tags.into_value(hidden_tags, "tags")? {
			body.insert("tags".into(), tags);
		}
		for (field, id) in [("staff", self.staff), ("groups", self.group)] {
			if let Some(id) = id {
				body.insert(field.into(), json!({ "values": [id] }));
			}
		}
		for (field, min, max) in self.ranges {
			let mut range = Map::new();
			for (bound, value) in [("min", min), ("max", max)] {
				if let Some(value) = value {
					range.insert(bound.into(), json!(value as i64));
				}
			}
			if !range.is_empty() {
				body.insert(field, Value::Object(range));
			}
		}

		let mut url = api_url(
			base_url,
			&format!("/search/series?page={}&size={size}", page - 1),
		);
		// Relevance means nothing without a title, so browsing falls back to the most viewed.
		let (sort, ascending) = match self.sort {
			"" if browsing => ("total_views", false),
			sort => (sort, self.ascending),
		};
		if !sort.is_empty() {
			let order = if ascending { "asc" } else { "desc" };
			url.push_str(&format!("&sort={sort},{order}"));
		}
		if self.exact_match {
			url.push_str("&exact_match=true");
		}
		Ok(Some(post(base_url, &url, &Value::Object(body))?))
	}
}
