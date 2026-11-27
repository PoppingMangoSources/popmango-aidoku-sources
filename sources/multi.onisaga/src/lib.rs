#![no_std]
mod api;
mod helpers;
mod home;
mod models;
mod rsc;
mod settings;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Listing, ListingProvider, Manga,
	MangaPageResult, Page, PageContent, Result, Source,
	alloc::{String, Vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};
use api::{BASE_URL, SORTS, SearchParams, get, html};
use core::cell::RefCell;
use helpers::{chapters, details, next_page, terms, title_url};
use models::Years;

struct OniSaga {
	// The site pages with signed cursors only.
	cursor: RefCell<Option<String>>,
}

impl OniSaga {
	fn page(&self, mut search: SearchParams, page: i32) -> Result<MangaPageResult> {
		if page > 1 {
			let Some(cursor) = self.cursor.take() else {
				return Ok(MangaPageResult::default());
			};
			search.cursor = Some(cursor);
		}
		let (result, next) = search.page()?;
		*self.cursor.borrow_mut() = next;
		Ok(result)
	}
}

impl Source for OniSaga {
	fn new() -> Self {
		Self {
			cursor: RefCell::new(None),
		}
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut search = SearchParams::new(query);
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						search.sort = sort;
					}
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"origin" => search.origin = Some(included),
					"genre" => {
						search.genres.include = included;
						search.genres.exclude.extend(excluded);
					}
					"status" => search.status = included,
					"form" => search.form = included,
					"demographic" => search.demographic = included,
					"rating" => search.ratings = included,
					_ => {}
				},
				FilterValue::Text { id, value } => match id.as_str() {
					"theme" => terms(
						&value,
						&mut search.themes.include,
						&mut search.themes.exclude,
					),
					"tag" => terms(&value, &mut search.tags.include, &mut search.tags.exclude),
					_ => {}
				},
				FilterValue::Range { id, from, to } if id == "year" => {
					search.year = Some(Years {
						from: from.map(|year| year as i32),
						to: to.map(|year| year as i32),
					});
				}
				FilterValue::Check { id, value } if id == "readable" => {
					search.readable = value == 1
				}
				_ => {}
			}
		}
		// A genre included in a search overrides its exclusion in settings.
		let included = &search.genres.include;
		search.genres.exclude.retain(|id| !included.contains(id));
		self.page(search, page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let languages = settings::languages();
		let first = languages.first().map_or("en", String::as_str);
		let doc = get(&title_url(&manga.key, first))?;
		if needs_details {
			manga.copy_from(details(&doc, &manga.key)?);
		}
		if !needs_chapters {
			return Ok(manga);
		}
		if needs_details && (languages.len() > 1 || next_page(&doc).is_some()) {
			send_partial_result(&manga);
		}

		// Each language has its own chapter pages.
		let others = Request::send_all(
			languages
				.iter()
				.skip(1)
				.map(|language| Request::get(title_url(&manga.key, language)))
				.collect::<core::result::Result<Vec<_>, _>>()?,
		);
		let docs =
			core::iter::once(Ok(doc)).chain(others.into_iter().map(|response| html(response?)));
		let mut list = Vec::new();
		for (doc, language) in docs.zip(&languages) {
			let mut doc = doc?;
			loop {
				chapters(&doc, &manga.key, language, &mut list);
				let Some(next) = next_page(&doc) else {
					break;
				};
				doc = get(&next)?;
			}
		}
		if languages.len() > 1 {
			// Stable, so each number keeps the reader's language order.
			list.sort_by(|a, b| {
				let number = |chapter: &Chapter| chapter.chapter_number.unwrap_or(-1.0);
				number(b).total_cmp(&number(a))
			});
		}
		manga.chapters = Some(list);
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let doc = get(&format!("{BASE_URL}/read/{}/{}", manga.key, chapter.key))?;
		if doc.select_first("main[data-ui=content-gate]").is_some() {
			bail!("OniSaga shows this chapter only to signed-in readers who allow its rating");
		}
		let pages = doc
			.select("figure[data-page] img")
			.into_iter()
			.flatten()
			.map(|img| {
				img.attr("src")
					.filter(|src| !src.is_empty())
					.map(|src| Page {
						content: PageContent::url(src),
						..Default::default()
					})
					.ok_or_else(|| error!("OniSaga left a page without an image"))
			})
			.collect::<Result<Vec<_>>>()?;
		if pages.is_empty() {
			bail!("OniSaga has no pages for this chapter");
		}
		Ok(pages)
	}
}

impl ListingProvider for OniSaga {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let Some(sort) = SORTS.iter().find(|sort| **sort == listing.id) else {
			bail!("Unknown listing");
		};
		self.page(SearchParams::sorted(sort), page)
	}
}

impl DeepLinkHandler for OniSaga {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let mut segments = path
			.split(['?', '#'])
			.next()
			.unwrap_or_default()
			.split('/')
			.filter(|segment| !segment.is_empty());
		Ok(match (segments.next(), segments.next(), segments.next()) {
			(Some("title"), Some(slug), None) => Some(DeepLinkResult::Manga { key: slug.into() }),
			(Some("read"), Some(slug), Some(release)) => Some(DeepLinkResult::Chapter {
				manga_key: slug.into(),
				key: release.into(),
			}),
			_ => None,
		})
	}
}

register_source!(OniSaga, Home, ListingProvider, DeepLinkHandler);

#[cfg(test)]
mod test;
