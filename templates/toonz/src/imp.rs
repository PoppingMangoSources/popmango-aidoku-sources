use crate::{
	Params,
	helpers::{
		RATINGS, SORTS, SearchParams, check, comic_id, details, narrowed, series_page, series_path,
	},
	home,
	models::{ChapterList, ReaderImage},
};
use aidoku::{
	Chapter, DeepLinkResult, FilterValue, HomeLayout, Listing, Manga, MangaPageResult, Page,
	PageContent, Result,
	alloc::{String, Vec, format, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

pub trait Impl {
	fn new() -> Self;

	fn params(&self) -> Params;

	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some((key, _)) = query
			.as_deref()
			.and_then(|url| url.trim().strip_prefix(params.base_url))
			.and_then(|path| series_path(params, path))
			&& let Ok(page) = series_page(params, &key)
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

		let mut search = SearchParams::new(params, query);
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						search.sort = sort;
					}
				}
				FilterValue::Select { id, value } if id == "status" && !value.is_empty() => {
					search.status = value
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"genre" => {
						search.genres.extend(included);
						search.excluded_genres.extend(excluded);
					}
					// The site's levels include those below, so the highest one picked applies.
					"rating" => {
						if let Some(level) = RATINGS
							.iter()
							.rev()
							.find(|level| included.iter().any(|value| value == *level))
						{
							search.content.rating = String::from(*level);
						}
					}
					"type" => search.content.types = narrowed(included, params.type_count),
					_ => {}
				},
				FilterValue::Text { id, value } if id == "author" => search.author = Some(value),
				_ => {}
			}
		}
		// A tag included in a search overrides its exclusion in settings.
		let genres = &search.genres;
		search.content.blocked.retain(|slug| !genres.contains(slug));

		search.page(params, page)
	}

	fn get_manga_list(
		&self,
		params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let Some(sort) = SORTS.iter().find(|sort| **sort == listing.id) else {
			bail!("Unknown listing");
		};
		SearchParams {
			sort,
			..SearchParams::new(params, None)
		}
		.page(params, page)
	}

	fn get_manga_update(
		&self,
		params: &Params,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let page = series_page(params, &manga.key)?;
		if needs_details {
			details(&page, &mut manga)?;
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let url = format!(
				"{}/api/comics/{}/chapters",
				params.base_url,
				comic_id(&page)?
			);
			let chapters = check(Request::get(url)?.send()?)?
				.get_json_owned::<ChapterList>()?
				.chapters;
			manga.chapters = Some(
				chapters
					.into_iter()
					.map(|chapter| chapter.into_chapter(params, &manga.key))
					.collect(),
			);
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!(
			"{}/{}/{}/chapter/{}",
			params.base_url, params.series_path, manga.key, chapter.key
		);
		// The reader's data, without the page around it.
		let body = check(Request::get(url)?.header("RSC", "1").send()?)?.get_string()?;
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
					"{}/uploads/{}",
					params.base_url,
					image.filename.trim_start_matches('/')
				)),
				..Default::default()
			})
			.collect())
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		home::get_home(params)
	}

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(url
			.strip_prefix(params.base_url)
			.and_then(|path| series_path(params, path))
			.map(|(key, chapter)| match chapter {
				Some(chapter) => DeepLinkResult::Chapter {
					manga_key: key,
					key: chapter,
				},
				None => DeepLinkResult::Manga { key },
			}))
	}
}
