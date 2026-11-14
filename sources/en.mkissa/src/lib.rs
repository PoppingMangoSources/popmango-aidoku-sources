#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeLayout, ImageRequestProvider,
	Listing, ListingProvider, Manga, MangaPageResult, Page, PageContent, PageContext, Result,
	Source,
	alloc::{String, Vec, vec},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};
use serde_json::{Map, Value, json};

mod helpers;
mod home;
mod models;
mod network;
mod settings;
mod webview;

use helpers::{
	content_rating, cover_url, decode_entities, has_next_page, parse_chapters, parse_status,
	strip_html,
};
use home::{Feed, browse_request};
use models::*;
use network::{fetch, parse};

const SORTS: [&str; 6] = [
	"Top",
	"Latest_Update",
	"Release_Year",
	"Name_ASC",
	"Name_DESC",
	"Popular",
];

struct Mkissa;

impl Source for Mkissa {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		// A pasted series link finds that series
		if let Some(id) = query
			.as_deref()
			.and_then(|query| query.strip_prefix(BASE_URL))
			.and_then(|path| path.strip_prefix("/manga/"))
			.and_then(|path| path.split(['/', '?', '#']).next())
			.filter(|id| !id.is_empty())
			&& let Ok(DetailsData {
				manga: Some(detail),
			}) = fetch::<DetailsData>(DETAILS_QUERY, json!({ "id": id }))
		{
			return Ok(MangaPageResult {
				entries: vec![Manga {
					key: id.into(),
					title: decode_entities(detail.english_name.as_deref().unwrap_or(&detail.name)),
					cover: cover_url(detail.thumbnail.as_deref()),
					..Default::default()
				}],
				has_next_page: false,
			});
		}

		let mut search = Map::new();
		let mut country = String::from("ALL");
		let mut included: Vec<String> = Vec::new();
		let mut excluded: Vec<String> = Vec::new();
		if let Some(query) = query {
			search.insert("query".into(), query.into());
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						search.insert("sortBy".into(), (*sort).into());
					}
				}
				FilterValue::Select { id, value } if id == "country" => country = value,
				// A tapped tag sends its name
				FilterValue::Select { id, value } if id == "genre" => included.push(value),
				FilterValue::MultiSelect {
					included: chosen,
					excluded: left_out,
					..
				} => {
					included.extend(chosen);
					excluded.extend(left_out);
				}
				_ => {}
			}
		}
		if !included.is_empty() {
			search.insert("genres".into(), included.into());
		}
		if !excluded.is_empty() {
			search.insert("excludeGenres".into(), excluded.into());
		}
		let data: SearchData =
			parse(browse_request(Value::Object(search), page, &country)?.send()?)?;
		Ok(MangaPageResult {
			has_next_page: has_next_page(page, data.mangas.page_info.total),
			entries: data.mangas.edges.into_iter().map(Manga::from).collect(),
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			let data: DetailsData = fetch(DETAILS_QUERY, json!({ "id": manga.key }))?;
			let detail = data.manga.ok_or(error!("Series not found"))?;
			let mut tags: Vec<String> = detail
				.genres
				.unwrap_or_default()
				.into_iter()
				.chain(detail.tags.unwrap_or_default())
				.map(|tag| tag.trim().into())
				.filter(|tag: &String| !tag.is_empty())
				.collect();
			tags.sort_unstable();
			tags.dedup();
			manga.title = decode_entities(
				detail
					.english_name
					.as_deref()
					.filter(|name| !name.trim().is_empty())
					.unwrap_or(&detail.name)
					.trim(),
			);
			manga.cover = cover_url(detail.thumbnail.as_deref()).or(manga.cover);
			manga.description = detail
				.description
				.as_deref()
				.map(strip_html)
				.filter(|description| !description.is_empty());
			manga.authors = detail
				.authors
				.map(|authors| {
					authors
						.into_iter()
						.map(|author| author.trim().into())
						.filter(|author: &String| !author.is_empty())
						.collect::<Vec<_>>()
				})
				.filter(|authors| !authors.is_empty());
			manga.status = parse_status(detail.status.as_deref());
			manga.content_rating = content_rating(&tags, detail.is_adult);
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(format!("{BASE_URL}/manga/{}", manga.key));
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let data: ChaptersData = fetch(
				CHAPTERS_QUERY,
				json!({ "id": manga.key, "showId": format!("manga@{}", manga.key) }),
			)?;
			manga.chapters = Some(parse_chapters(data, &manga.key));
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		Ok(webview::page_urls_via_webview(&manga.key, &chapter.key)?
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl Home for Mkissa {
	fn get_home(&self) -> Result<HomeLayout> {
		home::get_home()
	}
}

impl ListingProvider for Mkissa {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		Feed::from_id(&listing.id)
			.ok_or(error!("Unknown listing"))?
			.listing_page(page)
	}
}

impl ImageRequestProvider for Mkissa {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.header("Origin", BASE_URL))
	}
}

impl DeepLinkHandler for Mkissa {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url
			.strip_prefix(BASE_URL)
			.and_then(|path| path.strip_prefix("/manga/"))
		else {
			return Ok(None);
		};
		let mut segments = path.split(['/', '?', '#']);
		let Some(manga_key) = segments.next().filter(|key| !key.is_empty()) else {
			return Ok(None);
		};
		// Chapter paths read `chapter-12-sub`
		if let Some(number) = segments
			.next()
			.and_then(|segment| segment.strip_prefix("chapter-"))
			.map(|rest| rest.trim_end_matches("-sub"))
			.filter(|number| !number.is_empty())
		{
			return Ok(Some(DeepLinkResult::Chapter {
				manga_key: manga_key.into(),
				key: number.into(),
			}));
		}
		Ok(Some(DeepLinkResult::Manga {
			key: manga_key.into(),
		}))
	}
}

register_source!(
	Mkissa,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
