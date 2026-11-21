#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Manga,
	MangaPageResult, Page, PageContext, Result, Source,
	alloc::{String, Vec, vec},
	imports::{
		net::Request,
		std::{current_date, send_partial_result},
	},
	prelude::*,
};

mod helpers;
mod home;
mod models;
mod reader;
mod settings;

use helpers::{browse, chapter_list, get_html, parse_flight_value, parse_key, parse_response};
use models::ReaderProps;
use settings::{Catalogue, Groups};

const BASE_URL: &str = "https://webnex.cc";

struct WebNex;

impl Source for WebNex {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		// A link that fails to resolve is searched as text.
		if let Some(url) = query.as_deref().filter(|url| {
			url.strip_prefix(BASE_URL)
				.is_some_and(|path| path.starts_with('/'))
		}) && let Ok(Some(
			DeepLinkResult::Manga { key } | DeepLinkResult::Chapter { manga_key: key, .. },
		)) = self.handle_deep_link(url.into())
			&& let Ok(Manga {
				key,
				title,
				cover,
				content_rating,
				status,
				..
			}) = get_html(&format!("{BASE_URL}/manga/{key}"), None)
				.and_then(|doc| models::details(&doc, &key))
		{
			return Ok(MangaPageResult {
				entries: vec![Manga {
					key,
					title,
					cover,
					content_rating,
					status,
					..Default::default()
				}],
				has_next_page: false,
			});
		}
		browse(query.as_deref(), page, filters, Catalogue::load())
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/manga/{}", manga.key);
		let doc = get_html(&url, None)?;
		if doc.select_first("h1.series-title").is_none() {
			bail!("WebNex could not load this series");
		}
		if needs_details {
			manga.copy_from(models::details(&doc, &manga.key)?);
		}
		if needs_chapters {
			let doc = chapter_list(&doc)?;
			let last = doc
				.select("a[href*='page='][href$='#chapters']")
				.into_iter()
				.flatten()
				.filter_map(|el| {
					el.attr("href")?
						.split_once("page=")?
						.1
						.split(['&', '#'])
						.next()?
						.parse::<usize>()
						.ok()
				})
				.max()
				.unwrap_or(1);
			if needs_details && last > 1 {
				send_partial_result(&manga);
			}
			let groups = Groups::load();
			let now = current_date();
			let mut uploads = models::uploads(&doc, &groups, now)?;
			for start in (2..=last).step_by(4) {
				let reqs = (start..=last.min(start + 3))
					// The series url has no query of its own.
					.map(|page| Request::get(format!("{url}?page={page}")))
					.collect::<core::result::Result<Vec<_>, _>>()?;
				for response in Request::send_all(reqs) {
					let page = chapter_list(&parse_response(&response?)?)?;
					if page.select_first(".ch-row").is_none() {
						bail!("WebNex did not return the requested chapter page");
					}
					uploads.extend(models::uploads(&page, &groups, now)?);
				}
			}
			manga.chapters = Some(models::select_chapters(uploads, &groups));
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		reader::pages(&chapter.key)
	}
}

impl DeepLinkHandler for WebNex {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		// The app only passes this source's own links, so any host is fine here.
		let Some(path) = url
			.split_once("://")
			.and_then(|(_, rest)| rest.find('/').map(|slash| &rest[slash..]))
		else {
			return Ok(None);
		};
		if let Some(key) = parse_key(path, "/manga/") {
			return Ok(Some(DeepLinkResult::Manga { key: key.into() }));
		}
		let Some(key) = parse_key(path, "/read/") else {
			return Ok(None);
		};
		let reader = parse_flight_value::<ReaderProps>(
			&get_html(&format!("{BASE_URL}/read/{key}"), None)?,
			"{\"chapter\":",
		)
		.ok_or_else(|| error!("WebNex did not identify the chapter's series"))?;
		Ok(
			parse_key(&reader.series.href, "/manga/").map(|s| DeepLinkResult::Chapter {
				manga_key: s.into(),
				key: key.into(),
			}),
		)
	}
}

impl ImageRequestProvider for WebNex {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

register_source!(WebNex, Home, ImageRequestProvider, DeepLinkHandler);
