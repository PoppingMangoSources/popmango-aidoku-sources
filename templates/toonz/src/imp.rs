use crate::{
	Params,
	helpers::{
		SORTS, SearchParams, chapter_items, details, genres, narrowed, reader_images, series_page,
		series_path,
	},
	home,
};
use aidoku::{
	Chapter, DeepLinkResult, Filter, FilterValue, GroupSetting, HomeLayout, Listing, Manga,
	MangaPageResult, MultiSelectFilter, MultiSelectSetting, Page, PageContent, Result, Setting,
	alloc::{String, Vec, format, vec},
	imports::std::send_partial_result,
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
				FilterValue::Select { id, value } => match id.as_str() {
					"status" => search.status = Some(value),
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
			manga.chapters = Some(
				chapter_items(params, &page)?
					.into_iter()
					.map(|chapter| chapter.into_chapter(params, &manga.key))
					.collect(),
			);
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let images = match reader_images(params, &manga.key, &chapter.key)? {
			Some(images) => images,
			// The list view prints only a number, which misses a slug with a suffix ("45-wad21").
			None => {
				let Ok(number) = chapter.key.parse::<f32>() else {
					bail!("Chapter not found");
				};
				let page = series_page(params, &manga.key)?;
				let key = chapter_items(params, &page)?
					.into_iter()
					.find(|item| item.number() == Some(number))
					.map(|item| String::from(item.key()))
					.ok_or_else(|| error!("Chapter not found"))?;
				reader_images(params, &manga.key, &key)?
					.ok_or_else(|| error!("Chapter not found"))?
			}
		};
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

	fn get_dynamic_filters(&self, params: &Params) -> Result<Vec<Filter>> {
		let (names, slugs) = genres(params)?;
		Ok(vec![
			MultiSelectFilter {
				id: "genre".into(),
				title: Some("Tags".into()),
				is_genre: true,
				can_exclude: true,
				options: names.into_iter().map(Into::into).collect(),
				ids: Some(slugs.into_iter().map(Into::into).collect()),
				..Default::default()
			}
			.into(),
		])
	}

	// The genre list lives on the site, so excluded genres are offered from it.
	fn get_dynamic_settings(&self, params: &Params) -> Result<Vec<Setting>> {
		let (names, slugs) = genres(params)?;
		Ok(vec![
			GroupSetting {
				title: "Excluded Genres".into(),
				footer: Some(
					"Titles with an excluded genre are hidden everywhere. An included tag \
					 overrides this for one search."
						.into(),
				),
				items: vec![
					MultiSelectSetting {
						key: "excludedGenres".into(),
						title: "Excluded Genres".into(),
						values: slugs.into_iter().map(Into::into).collect(),
						titles: Some(names.into_iter().map(Into::into).collect()),
						refreshes: Some(vec!["content".into(), "listings".into()]),
						..Default::default()
					}
					.into(),
				],
				..Default::default()
			}
			.into(),
		])
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
