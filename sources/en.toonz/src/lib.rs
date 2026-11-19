#![no_std]
mod helpers;
mod home;
mod models;
mod settings;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Listing, ListingProvider, Manga,
	MangaPageResult, Page, PageContent, Result, Source,
	alloc::{String, Vec, format, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};
use helpers::{SORTS, SearchParams, comic_id, details, series_page, series_path};
use models::{ChapterList, ReaderImage};

pub const BASE_URL: &str = "https://toonz.to";

struct Toonz;

impl Source for Toonz {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some((key, _)) = query
			.as_deref()
			.and_then(|url| url.trim().strip_prefix(BASE_URL))
			.and_then(series_path)
			&& let Ok(page) = series_page(&key)
		{
			let mut manga = Manga {
				key,
				..Default::default()
			};
			if details(&page, &mut manga).is_ok() {
				return Ok(MangaPageResult {
					entries: vec![Manga {
						key: manga.key,
						title: manga.title,
						cover: manga.cover,
						content_rating: manga.content_rating,
						..Default::default()
					}],
					has_next_page: false,
				});
			}
		}

		let mut search = SearchParams::new(query);
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						search.sort = sort;
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => match id.as_str() {
					"status" => search.status = value,
					"rating" => search.content.rating = value,
					_ => {}
				},
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"genre" => {
						search.genres.extend(included);
						search.excluded_genres.extend(excluded);
					}
					"type" => search.content.types = included,
					_ => {}
				},
				FilterValue::Text { id, value } if id == "author" => search.author = Some(value),
				_ => {}
			}
		}
		// A tag included in a search overrides its exclusion in settings.
		let genres = &search.genres;
		search.content.blocked.retain(|slug| !genres.contains(slug));

		search.page(page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let page = series_page(&manga.key)?;
		if needs_details {
			details(&page, &mut manga)?;
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let url = format!("{BASE_URL}/api/comics/{}/chapters", comic_id(&page)?);
			let chapters = Request::get(url)?.json_owned::<ChapterList>()?.chapters;
			manga.chapters = Some(
				chapters
					.into_iter()
					.map(|chapter| chapter.into_chapter(&manga.key))
					.collect(),
			);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/comic/{}/chapter/{}", manga.key, chapter.key);
		// The reader's data, without the page around it.
		let body = Request::get(url)?.header("RSC", "1").string()?;
		let marker = r#""images":"#;
		let start = body
			.find(marker)
			.ok_or_else(|| error!("Chapter not found"))?;
		let images = serde_json::Deserializer::from_str(&body[start + marker.len()..])
			.into_iter::<Vec<ReaderImage>>()
			.next()
			.ok_or_else(|| error!("Chapter not found"))??;
		if images.is_empty() || images.iter().any(|image| image.filename.is_empty()) {
			bail!("This chapter has missing pages");
		}
		Ok(images
			.into_iter()
			.map(|image| Page {
				content: PageContent::url(format!(
					"{BASE_URL}/uploads/{}",
					image.filename.trim_start_matches('/')
				)),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for Toonz {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let Some(sort) = SORTS.iter().find(|sort| **sort == listing.id) else {
			bail!("Unknown listing");
		};
		SearchParams::sorted(sort).page(page)
	}
}

impl DeepLinkHandler for Toonz {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(url
			.strip_prefix(BASE_URL)
			.and_then(series_path)
			.map(|(key, chapter)| match chapter {
				Some(chapter) => DeepLinkResult::Chapter {
					manga_key: key,
					key: chapter,
				},
				None => DeepLinkResult::Manga { key },
			}))
	}
}

register_source!(Toonz, Home, ListingProvider, DeepLinkHandler);
