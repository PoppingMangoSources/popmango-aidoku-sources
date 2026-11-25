#![no_std]
mod api;
mod helpers;
mod home;
mod models;
mod settings;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Manga,
	MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, format, vec},
	helpers::string::PlainText,
	imports::net::Request,
	prelude::*,
};
use api::{API_URL, BASE_URL, SORTS, SearchParams, comic_page, get, parse, site_headers};
use helpers::{Link, encrypt_payload, parse_link};
use models::{Genres, NOVEL_PREFIX, Novel, NovelText, Pages, Response, chapter_list};

struct Swarm;

impl Source for Swarm {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let pasted = match query.as_deref().and_then(parse_link) {
			Some(Link::Comic(slug) | Link::Chapter(slug, ..)) => comic_page(&slug)
				.and_then(|page| Manga::try_from(page.manga))
				.ok(),
			Some(Link::Novel(slug) | Link::NovelChapter(slug, _)) => {
				get::<Response<Novel>>(&format!("{API_URL}/novel/{slug}"))
					.ok()
					.and_then(|novel| novel.data.into_manga())
			}
			None => None,
		};
		if let Some(manga) = pasted {
			return Ok(MangaPageResult {
				entries: vec![manga],
				has_next_page: false,
			});
		}

		let mut search = SearchParams::new(query);
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						search.sort = sort;
					}
				}
				FilterValue::Select { id, value } if id == "genre" && !value.is_empty() => {
					let genres = get::<Genres>(&format!("{API_URL}/manga/genres"))?.genres;
					if genres.contains(&value) {
						search.genres.push(value);
					} else {
						search.tags.push(value);
					}
				}
				FilterValue::MultiSelect { id, included, .. } => match id.as_str() {
					"genre" => search.genres.extend(included),
					"type" => search.content.types = Some(included),
					"rating" => {
						search.content.rating = match included.as_slice() {
							[only] => only.clone(),
							_ => "all".into(),
						}
					}
					_ => {}
				},
				FilterValue::Text { id, value } if id == "tags" => search.tags.extend(
					value
						.split(',')
						.map(str::trim)
						.filter(|tag| !tag.is_empty())
						.map(String::from),
				),
				FilterValue::Range { id, from, to } => match id.as_str() {
					"chapters" => search.chapters = (from, to),
					"year" => search.years = (from, to),
					_ => {}
				},
				_ => {}
			}
		}
		let content = &mut search.content;
		content
			.excluded_genres
			.retain(|genre| !search.genres.contains(genre));
		content.excluded_tags.retain(|tag| {
			!search
				.tags
				.iter()
				.any(|included| included.eq_ignore_ascii_case(tag))
		});

		search.page(page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if let Some(slug) = manga.key.strip_prefix(NOVEL_PREFIX) {
			let novel = get::<Response<Novel>>(&format!("{API_URL}/novel/{slug}"))?.data;
			let details = novel.into_details()?;
			if needs_details {
				manga.copy_from(details);
			} else {
				manga.chapters = details.chapters;
			}
			return Ok(manga);
		}

		let page = comic_page(&manga.key)?;
		if needs_details {
			manga.copy_from(page.manga.into_details()?);
		}
		if needs_chapters {
			manga.chapters = Some(chapter_list(
				page.chapters,
				&manga.key,
				&settings::languages(),
				&settings::blocked_uploaders(),
				&settings::deduplicate_chapters(),
				&settings::preferred_uploaders(),
			));
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if let Some(slug) = manga.key.strip_prefix(NOVEL_PREFIX) {
			let url = format!("{API_URL}/novel/{slug}/chapter/{}", chapter.key);
			let text = get::<Response<NovelText>>(&url)?
				.data
				.content
				.lines()
				.map(str::trim)
				.filter(|line| !line.is_empty())
				.map(|line| line.escape_markdown())
				.collect::<Vec<_>>()
				.join("\n\n");
			if text.is_empty() {
				bail!("This chapter has no text");
			}
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}

		let payload = encrypt_payload(&format!(r#"{{"chapterId":"{}"}}"#, chapter.key));
		let response = site_headers(Request::post(format!("{API_URL}/sp"))?)
			.header("Content-Type", "application/json")
			.body(format!(r#"{{"payload":"{payload}"}}"#))
			.send()?;
		let images = parse::<Pages>(response)?.chapter.data;
		// `/noah/fall.png` is the site's missing-page placeholder.
		if images.is_empty()
			|| images
				.iter()
				.any(|url| url.is_empty() || url.ends_with("/noah/fall.png"))
		{
			bail!("This upload has no pages; try another uploader");
		}
		Ok(images
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl ImageRequestProvider for Swarm {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for Swarm {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(match parse_link(&url) {
			Some(Link::Comic(key)) => Some(DeepLinkResult::Manga { key }),
			Some(Link::Novel(slug)) => Some(DeepLinkResult::Manga {
				key: format!("{NOVEL_PREFIX}{slug}"),
			}),
			Some(Link::NovelChapter(slug, key)) => Some(DeepLinkResult::Chapter {
				manga_key: format!("{NOVEL_PREFIX}{slug}"),
				key,
			}),
			Some(Link::Chapter(slug, number, rank)) => comic_page(&slug)?
				.chapters
				.into_iter()
				.find(|chapter| {
					chapter.number == number && chapter.rank.as_deref().unwrap_or("a") == rank
				})
				.map(|chapter| DeepLinkResult::Chapter {
					manga_key: slug,
					key: chapter.id,
				}),
			None => None,
		})
	}
}

register_source!(Swarm, Home, ImageRequestProvider, DeepLinkHandler);
