#![no_std]

use aidoku::{
	BaseUrlProvider, Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result,
	Source,
	alloc::{String, Vec, format, vec},
	imports::net::{Request, TimeUnit, set_rate_limit},
	prelude::*,
};
use serde_json::json;

mod filters;
mod helpers;
mod home;
mod models;
mod settings;

use helpers::*;
use models::*;

struct Kagane;

fn empty_page() -> MangaPageResult {
	MangaPageResult {
		entries: Vec::new(),
		has_next_page: false,
	}
}

fn search_page(base_url: &str, request: Option<(Request, Vec<String>)>) -> Result<MangaPageResult> {
	let Some((request, ratings)) = request else {
		return Ok(empty_page());
	};
	let (response, sources) = send_with_sources(base_url, request)?;
	let response: SearchResponse = parse(response)?;
	Ok(MangaPageResult {
		entries: response
			.content
			.into_iter()
			.filter(|series| allows(&ratings, series.rating()))
			.map(|series| series.into_manga(base_url, &sources))
			.collect(),
		has_next_page: !response.last,
	})
}

impl Source for Kagane {
	fn new() -> Self {
		set_rate_limit(2, 2, TimeUnit::Seconds);
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let base_url = settings::base_url();
		let mut params = SearchParams {
			query: query.map(|query| query.trim().into()).unwrap_or_default(),
			..Default::default()
		};
		let mut tapped: Option<String> = None;
		let mut typed_tags = String::new();

		for filter in filters {
			match filter {
				FilterValue::Sort {
					index, ascending, ..
				} => {
					params.sort = SORTS.get(index as usize).copied().unwrap_or_default();
					params.ascending = ascending;
				}
				FilterValue::Check { id, value } if id == "exact_match" => {
					params.exact_match = value == 1;
				}
				FilterValue::Select { id, value } => match id.as_str() {
					"genre" => tapped = Some(value),
					"genre_mode" => params.genres.match_any = value == "any",
					"tag_mode" => params.tags.match_any = value == "any",
					"disturbing" => {
						params.disturbing = match value.as_str() {
							"only" => Some(true),
							"exclude" => Some(false),
							_ => None,
						}
					}
					_ => {}
				},
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"genres" => {
						params.genres.included = included;
						params.genres.excluded = excluded;
					}
					"ratings" => params.ratings = included,
					"format" => params.formats = included,
					"status" => params.upload_statuses = included,
					"publication_status" => params.publication_statuses = included,
					"original_language" => params.original_languages = included,
					"sources" => params.sources = included,
					_ => {}
				},
				FilterValue::Text { id, value } => {
					let value = value.trim();
					if value.is_empty() {
						continue;
					}
					match id.as_str() {
						"tags" => typed_tags = value.into(),
						"author" => params.staff = Some(filters::staff_id(&base_url, value)?),
						"group" => params.group = Some(filters::group_id(&base_url, value)?),
						_ => {}
					}
				}
				FilterValue::Range { id, from, to } => params.ranges.push((id, from, to)),
				_ => {}
			}
		}

		// A tapped tag on a series page may be one of its genres or one of its tags.
		if let Some(name) = &tapped
			&& let Some(genre) = filters::genres(&base_url)?
				.into_iter()
				.find(|genre| genre.genre_name.eq_ignore_ascii_case(name))
		{
			params.genres.included.push(genre.id);
			tapped = None;
		}
		let browsing = params.query.is_empty();
		let hide_tags = browsing && !settings::hidden_tags().is_empty();
		if tapped.is_some() || !typed_tags.is_empty() || hide_tags {
			let tags = filters::tags(&base_url)?;
			if let Some(name) = tapped {
				let ids = filters::tag_ids(&tags, &name);
				if ids.is_empty() {
					bail!("Kagane has no genre or tag named {name}");
				}
				params.tags.included.extend(ids);
			}
			let (included, excluded) = filters::typed_tags(&tags, &typed_tags)?;
			params.tags.included.extend(included);
			params.tags.excluded.extend(excluded);
			if hide_tags {
				params.hidden_tags = filters::hidden_ids(&tags);
			}
		}

		search_page(&base_url, params.request(&base_url, page, PAGE_SIZE)?)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let base_url = settings::base_url();
		let show_source = settings::show_source();
		// Chapters are credited to the series' source, so they need its details and the sources.
		let mut requests = vec![get(
			&base_url,
			&api_url(&base_url, &format!("/series/{}", manga.key)),
		)?];
		if needs_chapters {
			requests.push(get(
				&base_url,
				&api_url(
					&base_url,
					&format!("/series/{}/books/index?fields=List", manga.key),
				),
			)?);
		}
		let needs_sources = needs_chapters || show_source;
		if needs_sources {
			requests.push(sources_request(&base_url)?);
		}
		let mut responses = Request::send_all(requests).into_iter();
		let mut next = || {
			responses
				.next()
				.ok_or_else(|| error!("Kagane sent no response"))
		};
		let detail: SeriesDetail = parse(next()??)?;
		let index: Option<BookIndex> = if needs_chapters {
			Some(parse(next()??)?)
		} else {
			None
		};
		let sources = if needs_sources {
			next()?.ok().map(helpers::sources).unwrap_or_default()
		} else {
			Vec::new()
		};

		if needs_details {
			let source = detail
				.source_id
				.as_deref()
				.filter(|_| show_source)
				.and_then(|id| sources.iter().find(|source| source.source_id == id));
			manga.title = format_title(
				&detail.title,
				source.map(|source| source.title.as_str()),
				detail
					.edition_info
					.as_deref()
					.filter(|_| settings::show_edition()),
			);
			manga.cover = detail
				.series_covers
				.first()
				.map(|cover| api_url(&base_url, &format!("/image/{}", cover.image_id)));
			manga.url = Some(format!("{base_url}/series/{}", manga.key));
			// The series' own status, or the uploads' when the site has none.
			manga.status = parse_status(
				detail
					.publication_status
					.as_deref()
					.or(detail.upload_status.as_deref())
					.unwrap_or_default(),
			);
			manga.viewer = parse_viewer(detail.format.as_deref());
			manga.content_rating =
				content_rating(rating(detail.age_rating.as_deref(), detail.is_pornographic));
			manga.description = detail
				.description
				.as_deref()
				.map(str::trim)
				.filter(|description| !description.is_empty())
				.map(Into::into);

			// A staff member credited for both story and art lands in both lists.
			let mut authors: Vec<String> = Vec::new();
			let mut artists: Vec<String> = Vec::new();
			for staff in detail.series_staff {
				let role = staff.role.to_lowercase();
				let is_author = role.contains("author") || role.contains("story");
				let is_artist = role.contains("artist") || role.contains("art");
				match (is_author, is_artist) {
					(true, true) => {
						authors.push(staff.name.clone());
						artists.push(staff.name);
					}
					(true, false) => authors.push(staff.name),
					(false, true) => artists.push(staff.name),
					(false, false) => {}
				}
			}
			manga.authors = (!authors.is_empty()).then_some(authors);
			manga.artists = (!artists.is_empty()).then_some(artists);

			let show_spoilers = settings::show_spoiler_tags();
			let tags: Vec<String> = detail
				.genres
				.into_iter()
				.filter(|genre| show_spoilers || !genre.is_spoiler)
				.map(|genre| genre.genre_name)
				.chain(
					detail
						.tags
						.into_iter()
						.filter(|tag| show_spoilers || !tag.is_spoiler)
						.map(|tag| tag.tag_name),
				)
				.collect();
			manga.tags = (!tags.is_empty()).then_some(tags);
		}

		if let Some(index) = index {
			let uploader = uploader(&sources, detail.source_id.as_deref());
			manga.chapters = Some(index.into_chapters(&base_url, &manga.key, uploader.as_deref()));
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let base_url = settings::base_url();
		let data_saver = settings::data_saver();
		let integrity: IntegrityResponse = parse(
			site_request(
				Request::post(format!("{base_url}/api/integrity"))?,
				&base_url,
			)
			.header("Content-Type", "application/json")
			.body(String::new())
			.send()?,
		)?;

		// The integrity token buys an access token and the page manifest.
		let challenge: ChallengeResponse = parse(
			post(
				&base_url,
				&api_url(
					&base_url,
					&format!("/books/{}?is_datasaver={data_saver}", chapter.key),
				),
				&json!({}),
			)?
			.header("x-integrity-token", &integrity.token)
			.send()?,
		)?;

		let cache_url = challenge
			.cache_url
			.filter(|url| !url.is_empty())
			.unwrap_or_else(|| base_url.clone());
		let token = challenge.access_token;
		let mut pages: Vec<(i32, Page)> = challenge
			.manifest
			.map(|manifest| manifest.pages)
			.unwrap_or_default()
			.into_iter()
			.map(|page| {
				let ext = page.ext.as_deref().unwrap_or("jxl");
				let url = format!(
					"{cache_url}/api/v2/books/page/{}/{}.{ext}?token={token}&is_datasaver={data_saver}",
					chapter.key, page.page_id
				);
				(
					page.page_no,
					Page {
						content: PageContent::url(url),
						..Default::default()
					},
				)
			})
			.collect();
		if pages.is_empty() {
			bail!("Kagane returned no pages for this chapter");
		}
		pages.sort_by_key(|(number, _)| *number);
		Ok(pages.into_iter().map(|(_, page)| page).collect())
	}
}

impl ListingProvider for Kagane {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let base_url = settings::base_url();
		let Some(sort) = home::listing_sort(&listing.id) else {
			bail!("Unknown listing");
		};
		let params = SearchParams {
			hidden_tags: filters::hidden_tag_ids(&base_url)?,
			..SearchParams::sorted(sort)
		};
		search_page(&base_url, params.request(&base_url, page, PAGE_SIZE)?)
	}
}

impl DeepLinkHandler for Kagane {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some((_, path)) = url.split_once("/series/") else {
			return Ok(None);
		};
		let path = path.split(['?', '#']).next().unwrap_or_default();
		let mut segments = path.split('/').filter(|segment| !segment.is_empty());
		let Some(segment) = segments.next() else {
			return Ok(None);
		};
		// The site can put a slug before the series id.
		let key = segment
			.len()
			.checked_sub(36)
			.and_then(|start| segment.get(start..))
			.filter(|id| id.split('-').map(str::len).eq([8, 4, 4, 4, 12]))
			.unwrap_or(segment);
		Ok(Some(match (segments.next(), segments.next()) {
			(Some("reader"), Some(chapter)) => DeepLinkResult::Chapter {
				manga_key: key.into(),
				key: chapter.into(),
			},
			_ => DeepLinkResult::Manga { key: key.into() },
		}))
	}
}

impl ImageRequestProvider for Kagane {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		let base_url = settings::base_url();
		Ok(Request::get(url)?
			.header("Origin", &base_url)
			.header("Referer", &format!("{base_url}/")))
	}
}

impl BaseUrlProvider for Kagane {
	fn get_base_url(&self) -> Result<String> {
		Ok(settings::base_url())
	}
}

register_source!(
	Kagane,
	ListingProvider,
	Home,
	DynamicFilters,
	DynamicSettings,
	DeepLinkHandler,
	ImageRequestProvider,
	BaseUrlProvider
);
