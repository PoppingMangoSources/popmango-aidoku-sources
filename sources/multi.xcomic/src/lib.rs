#![no_std]
extern crate alloc;

use core::cell::RefCell;

mod filters;
mod graphql;
mod helpers;
mod home;
mod models;
mod settings;

use aidoku::{
	BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result,
	Source,
	alloc::{String, Vec, vec},
	imports::{defaults::defaults_get, net::Request, std::send_partial_result},
	prelude::*,
};
use graphql::{BrowseParams, EXTENDED_PAGE_SIZE};
use helpers::{chapter_from_data, manga_from_data};

const DEFAULT_BASE_URL: &str = "https://xcomic.me";

struct XComic {
	latest_cursor: RefCell<Option<i64>>,
}

impl XComic {
	fn latest_page(
		&self,
		base_url: &str,
		params: &BrowseParams,
	) -> Result<(Vec<models::ComicData>, bool)> {
		let mut before = if params.page == 1 {
			None
		} else {
			*self.latest_cursor.borrow()
		};
		for _ in 0..10 {
			let response = graphql::latest_uploads_request(base_url, before)?.send()?;
			let (entries, next_cursor) = graphql::parse_latest_uploads(response, params)?;
			// Only the home page pairs these with their chapter.
			let comics: Vec<models::ComicData> =
				entries.into_iter().map(|(comic, _)| comic).collect();
			let has_next_page = next_cursor.is_some();
			*self.latest_cursor.borrow_mut() = next_cursor;
			if !comics.is_empty() || !has_next_page {
				return Ok((comics, has_next_page));
			}
			before = next_cursor;
		}
		Ok((Vec::new(), self.latest_cursor.borrow().is_some()))
	}

	/// One page of browse results, shared by search and listings.
	fn browse_page(&self, base_url: &str, mut params: BrowseParams) -> Result<MangaPageResult> {
		// These sorts surface placeholder entries with no cover, so ask for a
		// bigger page to leave enough behind after filtering.
		let skip_coverless = matches!(params.sortby.as_str(), "field_update" | "field_create");
		let (comics, has_next_page) = if params.can_use_latest_uploads() {
			self.latest_page(base_url, &params)?
		} else {
			if skip_coverless {
				params.size = EXTENDED_PAGE_SIZE;
			}
			let response = graphql::browse_request(base_url, &params)?.send()?;
			graphql::parse_browse(response, &params)?
		};
		let entries = comics
			.into_iter()
			.map(|comic| manga_from_data(comic, base_url))
			.filter(|manga| !skip_coverless || manga.cover.is_some())
			.collect();
		Ok(MangaPageResult {
			has_next_page,
			entries,
		})
	}
}

impl Source for XComic {
	fn new() -> Self {
		Self {
			latest_cursor: RefCell::new(None),
		}
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let base_url = self.get_base_url()?;
		// Quick open: a pasted comic url or an `id:<value>` query resolves directly.
		if let Some(key) = query.as_deref().and_then(helpers::comic_key_from_query) {
			let comic = graphql::fetch_comic(&base_url, &key)?;
			return Ok(MangaPageResult {
				entries: vec![manga_from_data(comic, &base_url)],
				has_next_page: false,
			});
		}
		let mut params = BrowseParams::new("field_score", page)?;
		params.word = query.unwrap_or_default();
		let mut types = Vec::new();
		let mut ratings = Vec::new();
		let mut translated = Vec::new();

		for filter in filters {
			match filter {
				FilterValue::Sort { id, index, .. } if id == "sort" => {
					if let Some(sort) = graphql::SORT_IDS.get(index as usize) {
						params.sortby = (*sort).into();
					}
				}
				FilterValue::Select { id, value } => match id.as_str() {
					"original_status" => params.original_status = value,
					"upload_status" => params.upload_status = value,
					"chapter_count" => params.chapter_count = value,
					"include_mode" => params.include_mode = value,
					"exclude_mode" => params.exclude_mode = value,
					_ => {}
				},
				FilterValue::Text { id, value } if id == "year" => {
					(params.year_min, params.year_max) = helpers::parse_year(&value);
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"genres" | "formats" => {
						params.included_genres.extend(included);
						params.excluded_genres.extend(excluded);
					}
					"types" => types.extend(included),
					"content_ratings" => ratings.extend(included),
					"demographics" => params.demographics.extend(included),
					"original_languages" => params.original_languages.extend(included),
					"translated_languages" => translated.extend(included),
					_ => {}
				},
				_ => {}
			}
		}

		if !types.is_empty() {
			params.types = types;
		}
		if !ratings.is_empty() {
			params.content_ratings = ratings;
		}
		// A filter choice overrides the languages picked in settings.
		if !translated.is_empty() {
			params.translated_languages = translated;
		}

		self.browse_page(&base_url, params)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let base_url = self.get_base_url()?;
		let comic = graphql::fetch_comic(&base_url, &manga.key)?;
		// A comic is published in a single language, so the chapter language comes
		// from the comic node rather than from the chapter entries themselves.
		let language = comic
			.translated_language
			.as_deref()
			.and_then(settings::normalize_language);
		if needs_details {
			let chapters = manga.chapters.take();
			manga = manga_from_data(comic, &base_url);
			manga.chapters = chapters;
			if needs_chapters {
				send_partial_result(&manga);
			}
		}
		if needs_chapters {
			manga.chapters = Some(
				graphql::fetch_chapters(&base_url, &manga.key)?
					.into_iter()
					.filter_map(|chapter| {
						chapter_from_data(chapter, &base_url, language.as_deref(), false)
					})
					.collect(),
			);
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let base_url = self.get_base_url()?;
		let pages: Vec<Page> = graphql::fetch_page_urls(&base_url, &chapter.key)?
			.into_iter()
			.map(|url| helpers::absolute_url(&base_url, &url))
			.filter(|url| !url.is_empty())
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect();
		if pages.is_empty() {
			bail!("No pages found for this chapter");
		}
		Ok(pages)
	}
}

impl ListingProvider for XComic {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let base_url = self.get_base_url()?;
		self.browse_page(&base_url, BrowseParams::new(&listing.id, page)?)
	}
}

impl ImageRequestProvider for XComic {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		let base_url = self.get_base_url()?;
		Ok(Request::get(url)?
			.header("Referer", &format!("{base_url}/"))
			.header("Origin", &base_url))
	}
}

impl DeepLinkHandler for XComic {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some((manga_key, chapter)) = helpers::parse_comic_url(&url) else {
			return Ok(None);
		};
		Ok(Some(match chapter {
			Some(key) => DeepLinkResult::Chapter { manga_key, key },
			None => DeepLinkResult::Manga { key: manga_key },
		}))
	}
}

impl BaseUrlProvider for XComic {
	fn get_base_url(&self) -> Result<String> {
		Ok(defaults_get::<String>("url")
			.filter(|url| !url.is_empty())
			.unwrap_or_else(|| DEFAULT_BASE_URL.into()))
	}
}

register_source!(
	XComic,
	Home,
	ListingProvider,
	DynamicFilters,
	DynamicSettings,
	ImageRequestProvider,
	DeepLinkHandler,
	BaseUrlProvider
);
