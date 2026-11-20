use crate::{
	BASE_URL,
	settings::{CONTENT_TYPES, Catalogue},
};
use aidoku::{
	FilterValue, Manga, MangaPageResult, Result,
	alloc::{String, Vec, collections::BTreeMap, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element, Html},
		js::WebView,
		net::{Request, Response},
		std::{current_date, parse_date_with_options, sleep},
	},
	prelude::*,
};
use chrono::{DateTime, Datelike};
use serde::de::DeserializeOwned;

pub fn get_html(url: &str, cookie: Option<&str>) -> Result<Document> {
	let mut request = Request::get(url)?;
	if let Some(value) = cookie {
		request = request.header("Cookie", value);
	}
	parse_response(&request.send()?)
}

pub fn parse_response(response: &Response) -> Result<Document> {
	if !(200..300).contains(&response.status_code()) {
		bail!("WebNex returned HTTP {}", response.status_code());
	}
	let doc = response.get_html()?;
	if doc
		.select_first("title")
		.and_then(|el| el.text())
		.as_deref()
		== Some("Opening series")
	{
		let url = response
			.get_url()
			.ok_or_else(|| error!("WebNex returned no series URL"))?;
		let view = WebView::new();
		view.load(Request::get(&url)?)?;
		for _ in 0..10 {
			if view
				.eval("document.querySelector('h1.series-title')?.textContent || ''")
				.is_ok_and(|s| !s.is_empty())
			{
				break;
			}
			sleep(1);
		}
		let html = view.eval("document.documentElement.outerHTML")?;
		return Ok(Html::parse_with_url(html, url)?);
	}
	Ok(doc)
}

pub fn parse_key<'a>(url: &'a str, prefix: &str) -> Option<&'a str> {
	let key = url
		.strip_prefix(prefix)?
		.split(['?', '#'])
		.next()?
		.trim_end_matches('/');
	(!key.is_empty()
		&& key
			.bytes()
			.all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'))
	.then_some(key)
}

pub fn text(el: &Element, selector: &str) -> Option<String> {
	el.select_first(selector)?.text().filter(|s| !s.is_empty())
}

pub fn chunks(doc: &Document) -> BTreeMap<String, Element> {
	doc.select("div[id^='S:']")
		.into_iter()
		.flatten()
		.filter_map(|el| Some((el.attr("id")?.strip_prefix("S:")?.into(), el)))
		.collect()
}

// React streams both whole entries and fragments inside an entry.
pub fn expand(row: &Element, chunks: &mut BTreeMap<String, Element>) -> Option<String> {
	let mut html = row.html().unwrap_or_default();
	for slot in row.select("template[id^='P:']").into_iter().flatten() {
		let id = slot.attr("id")?;
		if let Some(chunk) = chunks.remove(id.strip_prefix("P:")?) {
			html = html.replacen(&slot.outer_html()?, &expand(&chunk, chunks)?, 1);
		}
	}
	Some(html)
}

pub fn chapter_list(doc: &Document) -> Result<Document> {
	let section = doc
		.select_first("section#chapters")
		.ok_or_else(|| error!("WebNex did not return a chapter section"))?;
	let html = expand(&section, &mut chunks(doc))
		.ok_or_else(|| error!("WebNex did not return the chapter list"))?;
	Ok(Html::parse_with_url(html, BASE_URL)?)
}

pub fn card(el: &Element) -> Option<Manga> {
	let link = el.select_first("a.card-link, a.side-row")?;
	Some(Manga {
		key: parse_key(&link.attr("href")?, "/manga/")?.into(),
		title: text(&link, ".card-title, .side-title")?,
		cover: link
			.select_first("img.cover-img")
			.and_then(|img| img.attr("abs:src")),
		..Default::default()
	})
}

// Next.js can split a component's props across flight chunks.
pub fn parse_flight_value<T: DeserializeOwned>(doc: &Document, marker: &str) -> Option<T> {
	let mut flight = String::new();
	for script in doc.select("script").into_iter().flatten() {
		if let Some(data) = script.html()
			&& let Some(json) = data.trim().strip_prefix("self.__next_f.push([1,")
			&& let Some(chunk) = serde_json::Deserializer::from_str(json)
				.into_iter::<String>()
				.next()
				.and_then(|v| v.ok())
		{
			flight.push_str(&chunk);
		}
	}
	flight.match_indices(marker).find_map(|(i, _)| {
		serde_json::Deserializer::from_str(&flight[i..])
			.into_iter::<T>()
			.next()?
			.ok()
	})
}

pub fn date(el: &Element) -> Option<i64> {
	let raw = el.attr("datetime")?;
	let utc = raw.strip_suffix('Z')?.split('.').next()?.replace('T', " ");
	parse_date_with_options(utc, "yyyy-MM-dd HH:mm:ss", "en_US_POSIX", "UTC")
}

pub fn parse_relative_date(label: &str, now: i64) -> Option<i64> {
	if label == "just now" {
		return Some(now);
	}
	let s = label.strip_suffix(" ago")?;
	let idx = s.find(|c: char| !c.is_ascii_digit())?;
	let n = s[..idx].parse::<i64>().ok()?;
	let unit = match &s[idx..] {
		"m" => 60,
		"h" => 3600,
		"d" => 86400,
		"w" => 604800,
		"mo" => 2592000,
		"y" => 31536000,
		_ => return None,
	};
	now.checked_sub(n.checked_mul(unit)?)
}

pub fn search_params(
	query: Option<&str>,
	page: i32,
	filters: Vec<FilterValue>,
	prefs: &mut Catalogue,
) -> QueryParameters {
	let mut p = QueryParameters::new();
	let (mut chapters, mut min_chapters) = (None, None);
	let (mut years, mut released) = ((None, None), (None, None));
	if query.is_some() {
		p.push("q", query);
	}
	if page > 1 {
		p.push("page", Some(&page.to_string()));
	}
	for f in filters {
		match f {
			FilterValue::Sort { index, .. } => {
				if let Some(sort) = [
					"popular",
					"relevance",
					"updated",
					"year_newest",
					"year_oldest",
					"title",
				]
				.get(index as usize)
				{
					p.push("sort", Some(sort));
				}
			}
			FilterValue::MultiSelect {
				id,
				included,
				excluded,
			} => match id.as_str() {
				"rating" => prefs.ratings = included,
				"kind" => prefs.types = included,
				// The site matches genres, formats and themes as tags.
				"genre" | "format" | "theme" => {
					if id == "genre" {
						prefs.hidden_genres.clear();
					}
					for v in included {
						p.push("tag", Some(&v));
					}
					for v in excluded {
						p.push("xtag", Some(&v));
					}
				}
				"status" | "demographic" | "language" => {
					for v in included {
						p.push(&id, Some(&v));
					}
				}
				_ => {}
			},
			FilterValue::Select { id, value } if id == "genre" => {
				prefs
					.hidden_genres
					.retain(|s| !s.eq_ignore_ascii_case(&value));
				let kind = value.to_ascii_lowercase();
				if CONTENT_TYPES.contains(&kind.as_str()) {
					prefs.types = vec![kind];
				} else {
					p.push("tag", Some(&value));
				}
			}
			FilterValue::Select { id, value } if id == "match" => p.push(&id, Some(&value)),
			FilterValue::Select { id, value } if id == "chapters" => chapters = Some(value),
			FilterValue::Text { id, value } if id == "minChapters" => {
				min_chapters = value.trim().parse::<u32>().ok().map(|n| n.to_string())
			}
			FilterValue::Range { id, from, to } if id == "year" => {
				years = (from.map(|v| v as i32), to.map(|v| v as i32))
			}
			FilterValue::Select { id, value } if id == "released" => {
				let year = DateTime::from_timestamp(current_date(), 0).map_or(1970, |d| d.year());
				released = match value.as_str() {
					"this-year" => (Some(year), None),
					"last-5-years" => (Some(year - 4), None),
					"2010s" => (Some(2010), Some(2019)),
					"2000s" => (Some(2000), Some(2009)),
					"before-2000" => (None, Some(1999)),
					_ => (None, None),
				};
			}
			FilterValue::Text { id, value }
				if matches!(id.as_str(), "author" | "artist" | "studio") =>
			{
				p.push(&id, Some(&value))
			}
			_ => {}
		}
	}
	if let Some(n) = min_chapters.or(chapters) {
		p.push("chapters", Some(&n));
	}
	for (name, year) in [
		("from", years.0.or(released.0)),
		("to", years.1.or(released.1)),
	] {
		if let Some(year) = year {
			p.push(name, Some(&year.to_string()));
		}
	}
	for r in &prefs.ratings {
		p.push("rated", Some(r));
	}
	for kind in &prefs.types {
		p.push("kind", Some(kind));
	}
	if let Some(r) = prefs.ceiling() {
		p.push("rating", Some(r));
	}
	p
}

pub fn browse(
	query: Option<&str>,
	page: i32,
	filters: Vec<FilterValue>,
	mut prefs: Catalogue,
) -> Result<MangaPageResult> {
	let params = search_params(query, page, filters, &mut prefs);
	let Some(rating) = prefs.ceiling().filter(|_| !prefs.types.is_empty()) else {
		return Ok(Default::default());
	};
	let doc = get_html(
		&format!("{BASE_URL}/browse?{params}"),
		Some(&prefs.cookie(rating)),
	)?;
	if let Some(error) = doc.select_first(".notice-error").and_then(|e| e.text()) {
		bail!("{error}");
	}
	let section = doc
		.select_first("section.browse")
		.ok_or_else(|| error!("WebNex did not return the catalogue"))?;
	let html = expand(&section, &mut chunks(&doc))
		.ok_or_else(|| error!("WebNex returned an unreadable catalogue"))?;
	let doc = Html::parse_with_url(html, BASE_URL)?;
	Ok(MangaPageResult {
		entries: doc
			.select("li.card-item")
			.into_iter()
			.flatten()
			.filter_map(|el| card(&el))
			.collect(),
		has_next_page: doc.select_first("a[aria-label='Next page']").is_some(),
	})
}
