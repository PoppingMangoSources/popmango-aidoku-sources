#![no_std]
extern crate alloc;
mod flight;
mod models;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, Link, Listing, ListingProvider, Manga, MangaPageResult,
	MangaStatus, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, borrow::ToOwned, string::ToString, vec},
	helpers::uri::encode_uri_component,
	imports::{net::Request, std::parse_date},
	prelude::*,
};
use flight::extract_flight;
use models::{ApiManga, ApiMangaDetails, ApiMangaList, FlightChapterList, FlightImages, TagValue};

const BASE_URL: &str = "https://reimanga.net";
const PAGE_SIZE: i32 = 24;

const ADULT_GENRES: &[&str] = &["ecchi", "smut", "adult", "mature", "yaoi", "yuri", "hentai"];

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The catalogue addresses a series as "<slug>-<id>"; the id lives at the tail.
fn manga_id_for(manga: &ApiManga) -> String {
	match manga.name_url.as_deref() {
		Some(slug) if !slug.is_empty() => format!("{slug}-{}", manga.id),
		_ => manga.id.to_string(),
	}
}

/// The API wants the numeric id on its own, recovered from the tail of the key.
fn numeric_id_from(manga_id: &str) -> Option<String> {
	let digits: String = manga_id
		.chars()
		.rev()
		.take_while(|c| c.is_ascii_digit())
		.collect();
	if digits.is_empty() {
		None
	} else {
		Some(digits.chars().rev().collect())
	}
}

fn tag_names(values: Option<&Vec<TagValue>>) -> Vec<String> {
	values
		.map(|list| {
			list.iter()
				.filter_map(|value| value.name())
				.map(clean)
				.filter(|name| !name.is_empty())
				.collect()
		})
		.unwrap_or_default()
}

/// Listing rows carry genres as one comma-separated slug string while the detail
/// payload sends objects, so both spellings feed the same list.
fn genre_names(manga: &ApiManga) -> Vec<String> {
	let named = tag_names(manga.genres.as_ref());
	if !named.is_empty() {
		return named;
	}
	manga
		.genre_slugs
		.as_deref()
		.unwrap_or("")
		.split(',')
		.map(str::trim)
		.filter(|slug| !slug.is_empty())
		.map(|slug| {
			let spaced = slug.replace('-', " ");
			let mut chars = spaced.chars();
			let mut out = String::new();
			let mut capitalize = true;
			for c in chars.by_ref() {
				if capitalize && c.is_alphabetic() {
					out.extend(c.to_uppercase());
					capitalize = false;
				} else {
					out.push(c);
					if c == ' ' {
						capitalize = true;
					}
				}
			}
			out
		})
		.collect()
}

fn content_rating_for(manga: &ApiManga) -> ContentRating {
	if manga.is_adult.unwrap_or(0) != 0 {
		return ContentRating::NSFW;
	}
	let mut names = genre_names(manga);
	names.extend(tag_names(manga.tags.as_ref()));
	let adult = names
		.iter()
		.any(|name| ADULT_GENRES.contains(&name.to_lowercase().as_str()));
	if adult {
		ContentRating::NSFW
	} else {
		ContentRating::Suggestive
	}
}

/// Covers are served as webp; only the thumbnail path is ever handed out as png.
fn cover_for(manga: &ApiManga) -> String {
	if let Some(url) = manga
		.cover_url
		.as_deref()
		.map(str::trim)
		.filter(|u| !u.is_empty())
	{
		if url.contains("/covers/") && url.ends_with("/thumbnail.png") {
			return format!("{}.webp", url.trim_end_matches(".png"));
		}
		url.to_string()
	} else {
		format!("{BASE_URL}/covers/{}/thumbnail.webp", manga.id)
	}
}

/// Parses an ISO-8601 timestamp string to a unix timestamp.
fn parse_iso_date(raw: &str) -> Option<i64> {
	let trimmed = raw.trim();
	if trimmed.len() < 19 {
		return None;
	}
	parse_date(&trimmed[..19], "yyyy-MM-dd'T'HH:mm:ss")
}

fn status_for(manga: &ApiManga) -> MangaStatus {
	if manga.completed.unwrap_or(0) == 1 {
		MangaStatus::Completed
	} else {
		MangaStatus::Ongoing
	}
}

fn to_manga(manga: &ApiManga) -> Manga {
	let genres = {
		let mut list = genre_names(manga);
		list.extend(tag_names(manga.tags.as_ref()));
		let mut seen: Vec<String> = Vec::new();
		list.retain(|name| {
			let lower = name.to_lowercase();
			if seen.contains(&lower) {
				false
			} else {
				seen.push(lower);
				true
			}
		});
		list
	};
	let authors = tag_names(manga.authors.as_ref());
	Manga {
		key: manga_id_for(manga),
		title: clean(
			manga
				.title
				.as_deref()
				.or(manga.name.as_deref())
				.unwrap_or(""),
		),
		cover: Some(cover_for(manga)),
		authors: (!authors.is_empty()).then_some(authors),
		description: manga
			.description
			.as_deref()
			.map(clean)
			.filter(|d| !d.is_empty()),
		url: Some(format!("{BASE_URL}/manga/{}", manga_id_for(manga))),
		tags: (!genres.is_empty()).then_some(genres),
		status: status_for(manga),
		content_rating: content_rating_for(manga),
		..Default::default()
	}
}

fn fetch_json<T: serde::de::DeserializeOwned>(url: &str) -> Result<T> {
	Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.header("Accept", "application/json")
		.header("Cookie", "showAdultContent=true")
		.send()?
		.get_json_owned()
}

/// Chapter lists and reader pages live only in the route's server payload; this
/// header asks for that payload instead of the rendered page.
fn fetch_flight(url: &str) -> Result<String> {
	Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.header("RSC", "1")
		.header("Accept", "text/x-component,*/*")
		.header("Cookie", "showAdultContent=true")
		.send()?
		.get_string()
}

fn list_from(list: &ApiMangaList) -> Vec<Manga> {
	list.data
		.as_ref()
		.map(|items| items.iter().map(to_manga).collect())
		.unwrap_or_default()
}

fn build_search_url(query: &str, sort: Option<&str>, page: i32, filters: &[FilterValue]) -> String {
	let mut params: Vec<String> = vec![format!("page={page}"), format!("limit={PAGE_SIZE}")];
	let term = query.trim();
	if !term.is_empty() {
		params.push(format!("search={}", encode_uri_component(term)));
	}
	if let Some(sort) = sort {
		params.push(format!("sort={sort}"));
		params.push(format!(
			"order={}",
			if sort == "title" { "asc" } else { "desc" }
		));
	}

	let mut included: Vec<String> = Vec::new();
	let mut excluded: Vec<String> = Vec::new();
	for filter in filters {
		match filter {
			FilterValue::Select { id, value } if id == "status" && !value.is_empty() => {
				params.push(format!("status={}", encode_uri_component(value)));
			}
			FilterValue::MultiSelect {
				included: inc,
				excluded: exc,
				..
			} => {
				included.extend(inc.iter().cloned());
				excluded.extend(exc.iter().cloned());
			}
			_ => {}
		}
	}
	if !included.is_empty() {
		params.push(format!(
			"genre={}",
			encode_uri_component(included.join(","))
		));
	}
	if !excluded.is_empty() {
		params.push(format!(
			"excludeGenres={}",
			encode_uri_component(excluded.join(","))
		));
	}
	format!("{BASE_URL}/api/manga?{}", params.join("&"))
}

struct ReiManga;

impl Source for ReiManga {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.unwrap_or_default();
		let url = build_search_url(&query, None, page, &filters);
		let list: ApiMangaList = fetch_json(&url)?;
		let current = list
			.pagination
			.as_ref()
			.and_then(|p| p.current_page)
			.unwrap_or(page as i64);
		let total = list
			.pagination
			.as_ref()
			.and_then(|p| p.total_pages)
			.unwrap_or(current);
		Ok(MangaPageResult {
			entries: list_from(&list),
			has_next_page: current < total,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let Some(numeric) = numeric_id_from(&manga.key) else {
			return Ok(manga);
		};
		if needs_details {
			let details: ApiMangaDetails = fetch_json(&format!("{BASE_URL}/api/manga/{numeric}"))?;
			if let Some(api) = details.manga.as_ref() {
				let mut parsed = to_manga(api);
				parsed.key = manga.key.clone();
				parsed.chapters = manga.chapters.take();
				manga = parsed;
			}
		}
		if needs_chapters {
			let body = fetch_flight(&format!("{BASE_URL}/manga/{}", manga.key))?;
			manga.chapters = Some(parse_chapters(&body, &manga.key));
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let body = fetch_flight(&format!("{BASE_URL}/manga/{}/{}", manga.key, chapter.key))?;
		let images = extract_flight::<FlightImages>(&body, "images")
			.and_then(|data| data.images)
			.unwrap_or_default();
		let mut ordered = images;
		ordered.sort_by_key(|image| image.page_number.unwrap_or(0));
		let mut seen: Vec<String> = Vec::new();
		let pages = ordered
			.into_iter()
			.filter_map(|image| {
				let url = image
					.image_url
					.or(image.url)
					.map(|u| u.trim().to_string())
					.filter(|u| !u.is_empty())?;
				(!seen.contains(&url)).then(|| {
					seen.push(url.clone());
					Page {
						content: PageContent::url(url),
						..Default::default()
					}
				})
			})
			.collect();
		Ok(pages)
	}
}

fn parse_chapters(body: &str, manga_key: &str) -> Vec<Chapter> {
	let entries = extract_flight::<FlightChapterList>(body, "chapters")
		.and_then(|data| data.chapters)
		.unwrap_or_default();
	let total = entries.len();
	entries
		.into_iter()
		.enumerate()
		.filter_map(|(index, entry)| {
			let id = entry.id?;
			let name = clean(entry.name.as_deref().unwrap_or(""));
			let number = number_in(&name).unwrap_or((total - index) as f32);
			let title = (!name.is_empty() && !is_plain_chapter_label(&name)).then(|| name.clone());
			let date = entry
				.upload_date
				.as_deref()
				.or(entry.updated_at.as_deref())
				.or(entry.created_at.as_deref())
				.and_then(parse_iso_date);
			Some(Chapter {
				key: id.to_string(),
				title,
				chapter_number: Some(number),
				date_uploaded: date,
				url: Some(format!("{BASE_URL}/manga/{manga_key}/{id}")),
				language: Some("en".into()),
				..Default::default()
			})
		})
		.collect()
}

/// Finds the first decimal number embedded in a label.
fn number_in(text: &str) -> Option<f32> {
	let mut number = String::new();
	let mut started = false;
	for ch in text.chars() {
		if ch.is_ascii_digit() || (ch == '.' && started) {
			number.push(ch);
			started = true;
		} else if started {
			break;
		}
	}
	number.trim_matches('.').parse().ok()
}

/// True when a label is just `Ch. 12` / `Chapter 12` with nothing else.
fn is_plain_chapter_label(name: &str) -> bool {
	let lower = name.to_lowercase();
	let rest = lower
		.strip_prefix("chapter")
		.or_else(|| lower.strip_prefix("ch."))
		.or_else(|| lower.strip_prefix("ch"))
		.unwrap_or(&lower)
		.trim();
	!rest.is_empty()
		&& rest
			.chars()
			.all(|c| c.is_ascii_digit() || c == '.' || c == ' ')
}

impl Home for ReiManga {
	fn get_home(&self) -> Result<HomeLayout> {
		let trending: Result<Vec<ApiManga>> =
			fetch_json(&format!("{BASE_URL}/api/manga/trending?limit=10&full=1"));
		let most_read: Result<Vec<ApiManga>> = fetch_json(&format!(
			"{BASE_URL}/api/manga/most-read?limit=30&period=week"
		));
		let new_list: Result<ApiMangaList> =
			fetch_json(&format!("{BASE_URL}/api/manga/new?limit=12"));
		let latest: Result<ApiMangaList> =
			fetch_json(&format!("{BASE_URL}/api/manga/latest-updates?limit=18"));
		let top_rated: Result<ApiMangaList> = fetch_json(&format!(
			"{BASE_URL}/api/manga?page=1&limit={PAGE_SIZE}&sort=scored&order=desc"
		));

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Ok(list) = trending {
			let entries: Vec<Manga> = list.iter().map(to_manga).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Featured".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries,
						auto_scroll_interval: Some(6.0),
					},
				});
			}
		}

		if let Ok(list) = most_read {
			let entries: Vec<Link> = list.iter().map(|m| to_manga(m).into()).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Read This Week".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					},
				});
			}
		}

		for (title, list) in [("New Manga", new_list), ("Latest Updates", latest)] {
			if let Ok(list) = list {
				let entries: Vec<Link> = list_from(&list).into_iter().map(Into::into).collect();
				if !entries.is_empty() {
					components.push(HomeComponent {
						title: Some(title.into()),
						subtitle: None,
						value: HomeComponentValue::Scroller {
							entries,
							listing: None,
						},
					});
				}
			}
		}

		if let Ok(list) = top_rated {
			let entries: Vec<Link> = list_from(&list).into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Top Rated".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}

impl ListingProvider for ReiManga {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = build_search_url("", Some(&listing.id), page, &[]);
		let list: ApiMangaList = fetch_json(&url)?;
		let current = list
			.pagination
			.as_ref()
			.and_then(|p| p.current_page)
			.unwrap_or(page as i64);
		let total = list
			.pagination
			.as_ref()
			.and_then(|p| p.total_pages)
			.unwrap_or(current);
		Ok(MangaPageResult {
			entries: list_from(&list),
			has_next_page: current < total,
		})
	}
}

impl aidoku::ImageRequestProvider for ReiManga {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.header(
				"Accept",
				"image/avif,image/webp,image/apng,image/png,image/svg+xml,*/*;q=0.8",
			))
	}
}

impl DeepLinkHandler for ReiManga {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(rest) = url.split("/manga/").nth(1) else {
			return Ok(None);
		};
		let mut segments = rest.split(['?', '#']).next().unwrap_or("").split('/');
		let Some(slug) = segments.next().filter(|s| !s.is_empty()) else {
			return Ok(None);
		};
		if let Some(chapter) = segments.next().filter(|s| !s.is_empty()) {
			return Ok(Some(DeepLinkResult::Chapter {
				manga_key: slug.to_owned(),
				key: chapter.to_owned(),
			}));
		}
		Ok(Some(DeepLinkResult::Manga {
			key: slug.to_owned(),
		}))
	}
}

register_source!(
	ReiManga,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
