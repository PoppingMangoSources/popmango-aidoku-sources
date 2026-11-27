#![no_std]
mod api;
mod helpers;
mod home;
mod models;
mod settings;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, ImageRequestProvider, Listing,
	ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, collections::BTreeMap, format, vec},
	imports::{
		net::{Request, TimeUnit, set_rate_limit},
		std::{current_date, send_partial_result},
	},
	prelude::*,
};
use api::{BASE_URL, CHAPTER_PAGE_SIZE, SORTS, SearchParams, parse, rpc, set_age};
use core::cell::{Cell, RefCell};
use helpers::{
	DEMOGRAPHICS, FORMS, GENRES, Link, page_expired, page_fragment, parse_link, parse_terms,
	slugify,
};
use models::{
	Bootstrap, ChapterPage, ChapterUnit, Empty, IndexInput, PageImage, ReleaseInput, Scope,
	SelectInput, SelectPage, Selection, SlugInput, TermOption, WorkDetail, Years,
};
use settings::languages;

const MAX_CURSORS: usize = 64;

struct OniSaga {
	cursors: RefCell<Vec<(String, String)>>,
	age: Cell<Option<bool>>,
	pages: RefCell<Option<(String, Vec<PageImage>)>>,
}

impl OniSaga {
	pub(crate) fn sync_age(&self, adult: bool) -> Result<()> {
		if self.age.get() != Some(adult) {
			set_age(adult)?;
			self.age.set(Some(adult));
		}
		Ok(())
	}

	fn page(&self, mut search: SearchParams, page: i32) -> Result<MangaPageResult> {
		search.finalize();
		if search.is_empty() {
			return Ok(MangaPageResult::default());
		}
		self.sync_age(search.adult())?;
		let listing_key = search.key()?;
		if page > 1 {
			let key = format!("{page}:{listing_key}");
			let cursor = self
				.cursors
				.borrow()
				.iter()
				.find(|(k, _)| *k == key)
				.map(|(_, cursor)| cursor.clone());
			let Some(cursor) = cursor else {
				return Ok(MangaPageResult::default());
			};
			search.cursor = Some(cursor);
		}
		let (result, next) = search.page()?;
		if let Some(cursor) = next {
			let key = format!("{}:{listing_key}", page + 1);
			let mut cursors = self.cursors.borrow_mut();
			if let Some((_, existing)) = cursors.iter_mut().find(|(k, _)| *k == key) {
				*existing = cursor;
			} else {
				if cursors.len() >= MAX_CURSORS {
					cursors.remove(0);
				}
				cursors.push((key, cursor));
			}
		}
		Ok(result)
	}

	fn detail(&self, slug: &str) -> Result<WorkDetail> {
		let response = rpc("catalog/detail", SlugInput { slug })?.send()?;
		let status = response.status_code();
		if (status == 403 || status == 404) && self.age.get() != Some(true) {
			self.sync_age(true)?;
			return parse(rpc("catalog/detail", SlugInput { slug })?.send()?);
		}
		parse(response)
	}

	fn bootstrap(&self, release_id: &str) -> Result<Vec<PageImage>> {
		let response = rpc("reader/bootstrap", ReleaseInput { release_id })?.send()?;
		let status = response.status_code();
		let bootstrap: Bootstrap =
			if (status == 403 || status == 404) && self.age.get() != Some(true) {
				self.sync_age(true)?;
				parse(rpc("reader/bootstrap", ReleaseInput { release_id })?.send()?)?
			} else {
				parse(response)?
			};
		let pages = bootstrap.into_pages()?;
		*self.pages.borrow_mut() = Some((release_id.into(), pages.clone()));
		Ok(pages)
	}
}

fn resolve_tapped_tags(tapped: Vec<String>, search: &mut SearchParams) {
	let mut unknown = Vec::new();
	for name in tapped {
		let slug = slugify(&name);
		if slug.is_empty() {
			continue;
		}
		if GENRES.contains(&slug.as_str()) {
			search.genres.include.push(slug);
		} else if DEMOGRAPHICS.contains(&slug.as_str()) {
			search.demographic.push(slug);
		} else if FORMS.contains(&slug.as_str()) {
			search.form.push(slug);
		} else {
			unknown.push((name, slug));
		}
	}
	if unknown.is_empty() {
		return;
	}
	let taxonomy = rpc("catalog/taxonomy", Empty {})
		.and_then(|req| req.send().map_err(Into::into))
		.and_then(parse::<Vec<TermOption>>)
		.unwrap_or_default();
	for (name, slug) in unknown {
		let matched = taxonomy
			.iter()
			.find(|term| term.slug == slug || term.name.eq_ignore_ascii_case(&name));
		match matched.map(|term| (term.group.as_str(), term.slug.as_str())) {
			Some(("genre", found)) => search.genres.include.push(found.into()),
			Some(("theme", found)) => search.themes.include.push(found.into()),
			Some((_, found)) => search.tags.include.push(found.into()),
			None => search.tags.include.push(slug),
		}
	}
}

fn fetch_scope_units(work_id: &str, scope: &Scope, first: ChapterPage) -> Result<Vec<ChapterUnit>> {
	let mut units = first.items;
	let mut cursor = first.next_cursor;
	while let Some(next_cursor) = cursor {
		let page: ChapterPage = parse(
			rpc(
				"chapters/index",
				IndexInput {
					work_id,
					stream_id: &scope.stream_id,
					edition_id: scope.edition_id.as_deref(),
					lang: &scope.language,
					order: "desc",
					limit: CHAPTER_PAGE_SIZE,
					cursor: Some(&next_cursor),
				},
			)?
			.send()?,
		)?;
		if page.items.is_empty() {
			break;
		}
		units.extend(page.items);
		cursor = page.next_cursor;
	}
	Ok(units)
}

fn resolve_missing_selections(
	work_id: &str,
	scope: &Scope,
	units: &[ChapterUnit],
) -> Result<BTreeMap<String, Selection>> {
	let missing = units
		.iter()
		.filter(|unit| {
			unit.selection.is_none()
				&& unit.readable_candidate_count != Some(0)
				&& !unit.id.is_empty()
		})
		.map(|unit| unit.id.clone())
		.collect::<Vec<_>>();
	if missing.is_empty() {
		return Ok(BTreeMap::new());
	}
	let requests = missing
		.chunks(CHAPTER_PAGE_SIZE as usize)
		.map(|chunk| {
			rpc(
				"chapters/select",
				SelectInput {
					work_id,
					stream_id: &scope.stream_id,
					edition_id: scope.edition_id.as_deref(),
					chapter_ids: chunk,
					language: &scope.language,
					preference_version: 0,
				},
			)
		})
		.collect::<Result<Vec<_>>>()?;
	let mut map = BTreeMap::new();
	for response in Request::send_all(requests) {
		let page: SelectPage = parse(response?)?;
		for mut item in page.items {
			let chapter_id = core::mem::take(&mut item.chapter_id);
			map.insert(chapter_id, item);
		}
	}
	Ok(map)
}

impl Source for OniSaga {
	fn new() -> Self {
		set_rate_limit(3, 1, TimeUnit::Seconds);
		Self {
			cursors: RefCell::new(Vec::new()),
			age: Cell::new(None),
			pages: RefCell::new(None),
		}
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		if let Some(target) = query.as_deref().and_then(parse_link) {
			let slug = match target {
				Link::Title(slug) | Link::Chapter(slug, _) => slug,
			};
			if let Ok(detail) = self.detail(&slug)
				&& let Ok(manga) = detail.into_basic_manga()
			{
				return Ok(MangaPageResult {
					entries: vec![manga],
					has_next_page: false,
				});
			}
		}

		let mut search = SearchParams::new(query);
		let mut tapped = Vec::new();
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
						search.genres.include.extend(included);
						search.genres.exclude.extend(excluded);
					}
					"status" => search.status = included,
					"form" => search.form = included,
					"demographic" => search.demographic = included,
					"rating" => search.ratings = included,
					_ => {}
				},
				FilterValue::Select { id, value } if !value.is_empty() && id == "genre" => {
					tapped.push(value);
				}
				FilterValue::Text { id, value } => match id.as_str() {
					"theme" => parse_terms(
						&value,
						&mut search.themes.include,
						&mut search.themes.exclude,
					),
					"tag" => {
						parse_terms(&value, &mut search.tags.include, &mut search.tags.exclude)
					}
					_ => {}
				},
				FilterValue::Range { id, from, to } if id == "year" => {
					let from = from.map(|year| year as i32);
					let to = to.map(|year| year as i32);
					if let (Some(start), Some(end)) = (from, to)
						&& start > end
					{
						bail!("Start year cannot exceed end year");
					}
					if from.is_some() || to.is_some() {
						search.year = Some(Years { from, to });
					}
				}
				FilterValue::Check { id, value } if id == "readable" => {
					search.readable = value == 1
				}
				_ => {}
			}
		}
		if !tapped.is_empty() {
			resolve_tapped_tags(tapped, &mut search);
		}
		self.page(search, page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let mut detail = self.detail(&manga.key)?;
		let reading = core::mem::take(&mut detail.reading);

		if needs_details {
			manga.copy_from(Manga::try_from(detail)?);
		} else {
			if !detail.is_comic() {
				bail!("OniSaga novel entries are not supported");
			}
			manga.key = detail.slug;
		}
		if !needs_chapters {
			return Ok(manga);
		}

		let chosen_languages = languages();
		let scopes = reading.language_scopes(&chosen_languages);
		if scopes.is_empty() {
			manga.chapters = Some(Vec::new());
			return Ok(manga);
		}
		if needs_details {
			send_partial_result(&manga);
		}

		let initial_requests = scopes
			.iter()
			.map(|scope| {
				rpc(
					"chapters/index",
					IndexInput {
						work_id: &reading.work_id,
						stream_id: &scope.stream_id,
						edition_id: scope.edition_id.as_deref(),
						lang: &scope.language,
						order: "desc",
						limit: CHAPTER_PAGE_SIZE,
						cursor: None,
					},
				)
			})
			.collect::<Result<Vec<_>>>()?;

		let mut list = Vec::new();
		for (scope, response) in scopes.iter().zip(Request::send_all(initial_requests)) {
			let first: ChapterPage = parse(response?)?;
			let units = fetch_scope_units(&reading.work_id, scope, first)?;
			let mut selections = resolve_missing_selections(&reading.work_id, scope, &units)?;
			for unit in units {
				let fallback = selections.remove(&unit.id);
				list.extend(unit.into_chapters(
					&manga.key,
					&scope.language,
					&reading.groups,
					fallback,
				));
			}
		}

		if scopes.len() > 1 {
			// Stable, so each number keeps the reader's language order.
			list.sort_by(|a, b| {
				let number = |chapter: &Chapter| chapter.chapter_number.unwrap_or(-1.0);
				number(b).total_cmp(&number(a))
			});
		}
		manga.chapters = Some(list);
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let pages = self.bootstrap(&chapter.key)?;
		Ok(pages
			.into_iter()
			.map(|page| Page {
				content: PageContent::url(format!("{}#oni={}.{}", page.url, chapter.key, page.ord)),
				..Default::default()
			})
			.collect())
	}
}

impl ListingProvider for OniSaga {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let search = match listing.id.as_str() {
			"updated" => SearchParams::sorted("updated"),
			"new" => SearchParams::sorted("new"),
			"finished" => SearchParams {
				status: vec!["completed".into()],
				..SearchParams::sorted("updated")
			},
			"published" => SearchParams::sorted("published"),
			_ => bail!("Unknown listing"),
		};
		self.page(search, page)
	}
}

impl ImageRequestProvider for OniSaga {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		let resolved = if let Some((clean, release_id, ord)) = page_fragment(&url) {
			if page_expired(clean, current_date()) {
				let cached = self.pages.borrow().as_ref().and_then(|(id, pages)| {
					if id != release_id {
						return None;
					}
					let found = pages.iter().find(|page| page.ord == ord)?;
					(!page_expired(&found.url, current_date())).then(|| found.url.clone())
				});
				match cached {
					Some(fresh) => fresh,
					None => self
						.bootstrap(release_id)
						.ok()
						.and_then(|pages| {
							pages
								.into_iter()
								.find(|page| page.ord == ord)
								.map(|page| page.url)
						})
						.unwrap_or_else(|| clean.into()),
				}
			} else {
				clean.into()
			}
		} else {
			url
		};
		Ok(Request::get(resolved)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for OniSaga {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		Ok(parse_link(&url).map(|target| match target {
			Link::Title(key) => DeepLinkResult::Manga { key },
			Link::Chapter(manga_key, key) => DeepLinkResult::Chapter { manga_key, key },
		}))
	}
}

register_source!(
	OniSaga,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
