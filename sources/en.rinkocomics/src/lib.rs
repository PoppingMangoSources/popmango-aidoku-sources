#![no_std]
mod chapters;
mod helpers;
mod settings;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue,
	Home, HomeLayout, ImageRequestProvider, Listing, ListingProvider, Manga, MangaPageResult,
	MangaStatus, MultiSelectFilter, Page, PageContent, PageContext, Result, Source, Viewer,
	alloc::{String, Vec, collections::BTreeSet, vec},
	helpers::{string::PlainText, uri::QueryParameters},
	imports::{html::Html, net::Request, std::send_partial_result},
	prelude::*,
};
use chapters::{LOCK_SUFFIX, collect_chapters, decode_cw_pages, has_more_chapters};
use helpers::{
	BASE_URL, SORTS, abs_url, build_home_layout, clean, fetch_html, image_from, parse_browse_page,
	parse_status, path_of,
};

struct RinkoComics;

impl Source for RinkoComics {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut kind = String::from("comic");
		let mut qs = QueryParameters::new();
		if query.is_some() {
			qs.push("q", query.as_deref());
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						qs.set("sort", Some(sort));
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => match id.as_str() {
					"type" => kind = value,
					"genre" | "genres[]" => qs.push("genres[]", Some(&value)),
					_ => qs.set(&id, Some(&value)),
				},
				FilterValue::MultiSelect { id, included, .. } if id == "genres[]" => {
					for genre in included {
						qs.push("genres[]", Some(&genre));
					}
				}
				_ => {}
			}
		}
		let url = match (page <= 1, qs.is_empty()) {
			(true, true) => format!("{BASE_URL}/{kind}/"),
			(true, false) => format!("{BASE_URL}/{kind}/?{qs}"),
			(false, true) => format!("{BASE_URL}/{kind}/page/{page}/"),
			(false, false) => format!("{BASE_URL}/{kind}/page/{page}/?{qs}"),
		};
		Ok(parse_browse_page(&fetch_html(&url)?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let key = manga.key.trim_matches('/');
		let url = format!("{BASE_URL}/{key}/");
		let body = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.string()?;
		let html = Html::parse_with_url(&body, &url)?;
		let is_novel = key.starts_with("novel/");

		if needs_details {
			if let Some(title) = html
				.select_first(".comic-info-upper h1, h1.novel-title, main h1, h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = html
				.select_first("meta[property='og:image']")
				.and_then(|el| el.attr("content"))
				.map(|s| abs_url(&s))
				.filter(|s| !s.is_empty())
				.or_else(|| image_from(&html.select_first("main")?, "aside img, img"))
				.or_else(|| manga.cover.take());
			manga.description = html
				.select_first("#cwSynopsis, .comic-synopsis, .novel-synopsis")
				.and_then(|el| el.text())
				.or_else(|| {
					html.select_first("meta[property='og:description']")
						.and_then(|el| el.attr("content"))
				})
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());

			let mut authors: Vec<String> = html
				.select(".comic-graph > span")
				.into_iter()
				.flatten()
				.filter_map(|span| span.text())
				.map(|t| clean(&t))
				.filter(|name| !name.is_empty() && name != "•")
				.collect();
			authors.sort_unstable();
			authors.dedup();
			manga.authors = (!authors.is_empty()).then_some(authors);

			let mut status = MangaStatus::Unknown;
			let mut series_type = String::new();
			for dt in html.select("#main dt, main dt").into_iter().flatten() {
				let Some(label) = dt.text().map(|t| clean(&t)) else {
					continue;
				};
				let Some(value) = dt.next().and_then(|dd| dd.text()).map(|t| clean(&t)) else {
					continue;
				};
				if label.eq_ignore_ascii_case("status") && status == MangaStatus::Unknown {
					status = parse_status(&value);
				} else if label.eq_ignore_ascii_case("type") && series_type.is_empty() {
					series_type = value;
				}
			}
			manga.status = status;

			let mut tags: Vec<String> = html
				.select("#main a[href*='/comics_genres/'], main a[href*='/comics_genres/']")
				.into_iter()
				.flatten()
				.filter_map(|link| link.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.collect();
			tags.sort_unstable();
			tags.dedup();
			manga.viewer = if is_novel {
				Viewer::Unknown
			} else if series_type.eq_ignore_ascii_case("manga") {
				Viewer::RightToLeft
			} else {
				Viewer::Webtoon
			};
			manga.content_rating = ContentRating::Safe;
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url.clone());

			if needs_chapters && has_more_chapters(&html) {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			manga.chapters = Some(collect_chapters(
				&html,
				&body,
				&url,
				settings::show_locked_chapters(),
			));
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if chapter.locked || chapter.key.ends_with(LOCK_SUFFIX) {
			bail!("This chapter is locked on RinkoComics");
		}
		let path = path_of(&chapter.key);
		let url = format!("{BASE_URL}/{path}/");
		let html = fetch_html(&url)?;

		if let Some(prose) = html.select_first("#cwNovelText, #textContent, .novel-content") {
			let paragraphs: Vec<String> = prose
				.select("p, h2, h3, h4, blockquote")
				.into_iter()
				.flatten()
				.filter_map(|node| node.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.map(|t| t.escape_markdown())
				.collect();
			let text = if !paragraphs.is_empty() {
				paragraphs.join("\n\n")
			} else {
				prose
					.text()
					.map(|t| {
						t.lines()
							.map(clean)
							.filter(|line| !line.is_empty())
							.map(|line| line.escape_markdown())
							.collect::<Vec<_>>()
							.join("\n\n")
					})
					.unwrap_or_default()
			};
			if text.is_empty() {
				bail!("No novel text found for this episode");
			}
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}

		let mut context = PageContext::new();
		context.insert("Referer".into(), url);

		// SwiftSoup stores `<script id="cwPages">` as a DataNode (`data()`), whereas scraper uses `text()`.
		if let Some(encoded) = html
			.select_first("script#cwPages, #cwPages")
			.and_then(|el| el.data().or_else(|| el.html()).or_else(|| el.text()))
			.filter(|t| !t.trim().is_empty())
		{
			let pages = decode_cw_pages(&encoded)?;
			if !pages.is_empty() {
				return Ok(pages
					.into_iter()
					.map(|img_url| Page {
						content: PageContent::url_context(img_url, context.clone()),
						..Default::default()
					})
					.collect());
			}
		}

		let pages: Vec<Page> = html
			.select("img.chapter-image")
			.into_iter()
			.flatten()
			.filter_map(|img| {
				let src = img
					.attr("data-src")
					.or_else(|| img.attr("data-lazy-src"))
					.or_else(|| img.attr("src"))?;
				let cleaned = src.trim();
				(!cleaned.is_empty()).then(|| Page {
					content: PageContent::url_context(abs_url(cleaned), context.clone()),
					..Default::default()
				})
			})
			.collect();
		if pages.is_empty() {
			bail!("No pages found for this chapter");
		}
		Ok(pages)
	}
}

impl DynamicFilters for RinkoComics {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let html = fetch_html(&format!("{BASE_URL}/comic/"))?;
		let mut seen = BTreeSet::new();
		let pairs: Vec<(String, String)> = html
			.select("#cwBrowse input[name]")
			.into_iter()
			.flatten()
			.filter_map(|input| {
				if input.attr("name").as_deref() != Some("genres[]") {
					return None;
				}
				let id = input
					.attr("value")
					.map(|s| clean(&s))
					.filter(|s| !s.is_empty())?;
				let label = input
					.next()
					.or_else(|| input.parent()?.select_first("span"))?
					.text()
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())?;
				seen.insert(id.clone()).then_some((label, id))
			})
			.collect();
		let (options, ids): (Vec<_>, Vec<_>) = pairs
			.into_iter()
			.map(|(opt, id)| (opt.into(), id.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genres[]".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: false,
				uses_tag_style: true,
				options,
				ids: Some(ids),
				..Default::default()
			}
			.into(),
		])
	}
}

impl ListingProvider for RinkoComics {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let (kind, sort) = match listing.id.as_str() {
			"updated" => ("comic", 1),
			"views" => ("comic", 2),
			"new" => ("comic", 3),
			"novels" => ("novel", 3),
			"novels-updated" => ("novel", 1),
			_ => ("comic", 1),
		};
		self.get_search_manga_list(
			None,
			page,
			vec![
				FilterValue::Select {
					id: "type".into(),
					value: kind.into(),
				},
				FilterValue::Sort {
					id: "sort".into(),
					index: sort,
					ascending: false,
				},
			],
		)
	}
}

impl Home for RinkoComics {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			Request::get(format!("{BASE_URL}/?show=today"))?
				.header("Referer", &format!("{BASE_URL}/")),
			Request::get(format!("{BASE_URL}/comic/?sort=new"))?
				.header("Referer", &format!("{BASE_URL}/")),
			Request::get(format!("{BASE_URL}/?rel=novels"))?
				.header("Referer", &format!("{BASE_URL}/")),
		];
		let [home_resp, new_comics_resp, novel_home_resp]: [_; 3] = Request::send_all(requests)
			.try_into()
			.map_err(|_| error!("Unexpected response count"))?;
		let home = home_resp?.get_html()?;
		let new_comics = new_comics_resp?.get_html()?;
		let novel_home = novel_home_resp?.get_html()?;
		Ok(build_home_layout(
			&home,
			&new_comics,
			&novel_home,
			settings::show_locked_chapters(),
		))
	}
}

impl ImageRequestProvider for RinkoComics {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for RinkoComics {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(rest) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let path = rest
			.split(['?', '#'])
			.next()
			.unwrap_or("")
			.trim_matches('/');
		if path.starts_with("comic/") || path.starts_with("novel/") {
			return Ok(Some(DeepLinkResult::Manga { key: path.into() }));
		}
		if path.starts_with("chapter/") || path.starts_with("novel_chapter/") {
			let html = fetch_html(&url)?;
			let selector = if path.starts_with("novel_chapter/") {
				"main a[href*='/novel/'], a[href*='/novel/']"
			} else {
				"main a[href*='/comic/'], a[href*='/comic/']"
			};
			if let Some(manga_href) = html.select_first(selector).and_then(|a| a.attr("href")) {
				let manga_key = path_of(&manga_href);
				if !manga_key.is_empty() {
					return Ok(Some(DeepLinkResult::Chapter {
						manga_key,
						key: path.into(),
					}));
				}
			}
		}
		Ok(None)
	}
}

register_source!(
	RinkoComics,
	Home,
	ListingProvider,
	DynamicFilters,
	ImageRequestProvider,
	DeepLinkHandler
);
