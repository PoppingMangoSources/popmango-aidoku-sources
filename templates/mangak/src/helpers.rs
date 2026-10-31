use crate::models::NextData;
use aidoku::{
	Listing, MangaStatus, Result,
	alloc::string::{String, ToString},
	helpers::uri::QueryParameters,
	imports::net::Request,
	prelude::*,
};
use serde::de::DeserializeOwned;

// Best Match sends no sort, as on the site
pub const SORTS: [Option<&str>; 11] = [
	None,
	Some("latest"),
	Some("newest"),
	Some("popular"),
	Some("rating"),
	Some("views_today"),
	Some("views_7days"),
	Some("views_30days"),
	Some("views"),
	Some("chapters"),
	Some("alphabetical"),
];

pub fn next_data<T: DeserializeOwned>(url: &str) -> Result<T> {
	let html = Request::get(url)?.html()?;
	let script = html
		.select_first("script#__NEXT_DATA__")
		.ok_or_else(|| error!("Missing __NEXT_DATA__"))?;
	let json = script
		.data()
		.or_else(|| script.html())
		.ok_or_else(|| error!("Empty __NEXT_DATA__"))?;
	Ok(serde_json::from_str::<NextData<T>>(&json)?.props.page_props)
}

pub fn listing_query(id: &str) -> QueryParameters {
	let mut qs = QueryParameters::new();
	match id {
		"popular_updates" => {
			qs.push("sort", Some("bookmarks"));
			qs.push("window", Some("today"));
		}
		"new" => qs.push("sort", Some("newest")),
		"uncensored" | "raw" => qs.push(&format!("is_{id}"), Some("true")),
		_ => qs.push("sort", Some("latest")),
	}
	qs
}

pub fn listing(id: &str, name: &str) -> Option<Listing> {
	Some(Listing {
		id: id.to_string(),
		name: name.to_string(),
		..Default::default()
	})
}

pub fn manga_status(status: Option<String>) -> MangaStatus {
	match status.unwrap_or_default().to_lowercase().as_str() {
		"ongoing" => MangaStatus::Ongoing,
		"completed" => MangaStatus::Completed,
		"hiatus" => MangaStatus::Hiatus,
		"cancelled" | "dropped" => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}
