#![no_std]
mod api;
mod helpers;
mod home;
mod models;
mod settings;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Listing,
	ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, vec},
	helpers::string::PlainText,
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};
use api::{API_URL, BASE_URL, SORTS, SearchParams, get};
use helpers::{Link, parse_link, site_languages};
use models::{ChapterDetail, ChapterItem, Data, Paged, Title};

struct MangaBall;

impl Source for MangaBall {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some(Link::Title(id)) = query.as_deref().and_then(parse_link)
			&& let Ok(found) = get::<Data<Title>>(&format!("{API_URL}/title/detail/{id}"))
		{
			return Ok(MangaPageResult {
				entries: vec![found.data.into()],
				has_next_page: false,
			});
		}

		let mut search = SearchParams::new(query);
		for filter in filters {
			match filter {
				FilterValue::Sort {
					index, ascending, ..
				} => {
					search.sort = SORTS.get(index as usize).copied().flatten();
					search.ascending = ascending;
				}
				FilterValue::Select { id, value } => match id.as_str() {
					"type" => search.kind = value,
					"status" => search.status = value,
					"demographic" => search.demographic = value,
					"original_language" => search.original_language = value,
					"tag_mode" => search.tag_mode = value,
					_ => {}
				},
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"rating" => {
						search.adult_mode = match included.as_slice() {
							[rating] if rating == "adult" => "only_18",
							[_] => "no_18",
							_ => "all",
						}
					}
					"genre" | "theme" | "format" | "content" => {
						search.included_tags.extend(included);
						search.excluded_tags.extend(excluded);
					}
					_ => {}
				},
				_ => {}
			}
		}
		// A tag included in a search overrides its exclusion in settings.
		let included = &search.included_tags;
		search.excluded_tags.retain(|id| !included.contains(id));

		search.page(page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			let url = format!("{API_URL}/title/detail/{}", manga.key);
			let mut details = get::<Data<Title>>(&url)?.data.into_details();
			details.description = details
				.description
				.map(|description| description.escape_markdown());
			manga.copy_from(details);
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let languages = site_languages(settings::languages());
			let codes = languages
				.iter()
				.map(|(code, _)| code.as_str())
				.collect::<Vec<_>>()
				.join(",");
			let excluded = settings::excluded_scanlators();
			let mut chapters = Vec::new();
			let mut page = 1;
			loop {
				let response: Paged<ChapterItem> = get(&format!(
					"{API_URL}/title/chapter-listing?title_id={}&page={page}&limit=1000&group_by=flat&sort_order=desc&language={codes}",
					manga.key
				))?;
				chapters.extend(
					response
						.data
						.into_iter()
						.filter(|chapter| {
							!chapter.group_name.as_deref().is_some_and(|group| {
								excluded.contains(&group.trim().to_lowercase())
							})
						})
						.filter_map(|chapter| chapter.into_chapter(&languages)),
				);
				if page >= response.pagination.total_pages {
					break;
				}
				page += 1;
			}
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{API_URL}/chapter-detail?chapter_id={}", chapter.key);
		let pages = get::<Data<ChapterDetail>>(&url)?.data.chapter.pages;
		if pages.is_empty() {
			bail!("This chapter has no pages");
		}
		Ok(pages
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for MangaBall {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let Some(sort) = SORTS.iter().flatten().find(|sort| **sort == listing.id) else {
			bail!("Unknown listing");
		};
		SearchParams::sorted(sort).page(page)
	}
}

impl ImageRequestProvider for MangaBall {
	// The image hosts refuse requests the site didn't refer.
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaBall {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(match parse_link(&url) {
			Some(Link::Title(key)) => Some(DeepLinkResult::Manga { key }),
			Some(Link::Chapter(key)) => {
				let url = format!("{API_URL}/chapter-detail?chapter_id={key}");
				get::<Data<ChapterDetail>>(&url)?
					.data
					.chapter
					.title_id
					.map(|manga_key| DeepLinkResult::Chapter { manga_key, key })
			}
			None => None,
		})
	}
}

register_source!(
	MangaBall,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
