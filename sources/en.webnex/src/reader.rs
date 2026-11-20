use crate::{
	BASE_URL,
	helpers::{parse_flight_value, parse_response, script_cookie},
	models::{ReaderPages, ReaderProps},
};
use aidoku::{Page, Result, alloc::Vec, imports::net::Request, prelude::*};

pub fn pages(chapter_key: &str) -> Result<Vec<Page>> {
	let url = format!("{BASE_URL}/read/{chapter_key}");
	// The page list needs the cookie the chapter page writes only for a page request.
	let doc = parse_response(
		&Request::get(&url)?
			.header("Accept", "text/html,application/xhtml+xml")
			.send()?,
	)?;
	let Some(reader) = parse_flight_value::<ReaderProps>(&doc, "{\"chapter\":") else {
		bail!(
			"{}",
			doc.select_first("section.state .notice-error, body > pre")
				.and_then(|el| el.text())
				.as_deref()
				.unwrap_or("WebNex did not provide the chapter's page list")
		);
	};
	if !reader.pages_url.starts_with('/') {
		bail!("WebNex returned an invalid page list address");
	}
	let cookie = script_cookie(&doc).ok_or_else(|| error!("WebNex did not open the chapter"))?;
	let data: ReaderPages = Request::get(format!("{BASE_URL}{}", reader.pages_url))?
		.header("Cookie", &cookie)
		.header("Referer", &url)
		.json_owned()?;
	data.try_into()
}
