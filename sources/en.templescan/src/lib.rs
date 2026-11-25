#![no_std]
mod helpers;
mod rsc;
mod settings;
#[cfg(test)]
mod test;
mod tokens;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeLayout, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result,
	Source,
	alloc::{String, Vec, vec},
	imports::net::Request,
	prelude::*,
};
use helpers::{
	BASE_URL, build_home_layout, build_request, decode_reader_tokens, extract_reader_tokens,
	fetch_doc, page_catalog, parse_catalog, parse_comic_href, parse_series_chapters,
	parse_series_details, sort_catalog,
};

struct TempleScan;

impl Source for TempleScan {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.map(|q| q.to_ascii_lowercase());
		let mut sort = 0;
		let mut status = None;
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => sort = index,
				FilterValue::Select { id, value } if id == "status" && !value.is_empty() => {
					status = Some(value);
				}
				_ => {}
			}
		}
		let doc = fetch_doc(&format!("{BASE_URL}/comics"))?;
		let mut items = parse_catalog(&doc);
		if items.is_empty() {
			bail!("No series catalog found on Temple Scan");
		}
		items.retain(|series| {
			if let Some(ref q) = query {
				let title = series.title.to_ascii_lowercase();
				let alt = series
					.alternative_names
					.as_deref()
					.map(str::to_ascii_lowercase);
				if !title.contains(q) && !alt.is_some_and(|a| a.contains(q)) {
					return false;
				}
			}
			if let Some(ref want) = status
				&& !series
					.status
					.as_deref()
					.is_some_and(|s| s.eq_ignore_ascii_case(want))
			{
				return false;
			}
			true
		});
		sort_catalog(&mut items, sort);
		Ok(page_catalog(items, page))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let doc = fetch_doc(&format!("{BASE_URL}/comic/{}", manga.key))?;
		if needs_details {
			parse_series_details(&doc, &mut manga);
		}
		if needs_chapters {
			manga.chapters = Some(parse_series_chapters(&doc, &manga.key));
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if chapter.locked {
			bail!("This chapter is early access for subscribers on Temple Scan");
		}
		let doc = fetch_doc(&format!("{BASE_URL}/comic/{}/{}", manga.key, chapter.key))?;
		if doc.select("main p").into_iter().flatten().any(|p| {
			p.text()
				.is_some_and(|t| t.to_ascii_lowercase().contains("chapter is early access"))
		}) {
			bail!("This chapter is early access for subscribers on Temple Scan");
		}

		let Some(reader) = extract_reader_tokens(&doc) else {
			bail!("No pages found for chapter {}", chapter.key);
		};
		let urls = match decode_reader_tokens(&reader.tokens, tokens::SEED) {
			Some(urls) => urls,
			None => {
				let Some((module, chunk)) = reader.script else {
					bail!("Temple Scan changed its reader encoding");
				};
				let body = build_request(&format!("{BASE_URL}/_next/{chunk}"))?.string()?;
				let Some(seed) = tokens::seed_from_chunk(&body, module) else {
					bail!("Temple Scan changed its reader encoding");
				};
				decode_reader_tokens(&reader.tokens, &seed)
					.ok_or_else(|| error!("Temple Scan changed its reader encoding"))?
			}
		};
		if urls.is_empty() {
			bail!("No pages found for chapter {}", chapter.key);
		}
		Ok(urls
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl Home for TempleScan {
	fn get_home(&self) -> Result<HomeLayout> {
		let doc = fetch_doc(&format!("{BASE_URL}/"))?;
		Ok(build_home_layout(&doc))
	}
}

impl ListingProvider for TempleScan {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let index = match listing.id.as_str() {
			"views" => 0,
			"updated" => 1,
			"created" => 2,
			_ => bail!("Unknown listing"),
		};
		self.get_search_manga_list(
			None,
			page,
			vec![FilterValue::Sort {
				id: "sort".into(),
				index,
				ascending: false,
			}],
		)
	}
}

impl ImageRequestProvider for TempleScan {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for TempleScan {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let Some((slug, chapter)) = parse_comic_href(path) else {
			return Ok(None);
		};
		Ok(Some(match chapter {
			Some(key) => DeepLinkResult::Chapter {
				manga_key: slug,
				key,
			},
			None => DeepLinkResult::Manga { key: slug },
		}))
	}
}

register_source!(
	TempleScan,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
