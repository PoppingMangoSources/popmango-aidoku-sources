#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue, Home,
	HomeLayout, ImageRequestProvider, Listing, ListingProvider, Manga, MangaPageResult,
	MultiSelectFilter, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, vec},
	imports::{
		net::Request,
		std::{current_date, send_partial_result},
	},
	prelude::*,
};
use serde_json::{Map, Value, json};

mod graphql;
mod helpers;
mod home;
mod models;
mod settings;
mod webview;

use graphql::*;
use helpers::{
	cover_url, day_range, display_title, has_next_page, parse_chapters, season_and_year,
};
use home::{Feed, browse_request};
use models::*;

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
			&& let Ok(title) = display_title(detail.english_name.as_deref(), &detail.name)
		{
			return Ok(MangaPageResult {
				entries: vec![Manga {
					key: id.into(),
					title,
					cover: cover_url(detail.thumbnail.as_deref()),
					..Default::default()
				}],
				has_next_page: false,
			});
		}

		let mut search = Map::new();
		let mut country = String::from("ALL");
		let mut genres: Vec<String> = Vec::new();
		let mut tags: Vec<String> = Vec::new();
		let mut excluded_genres: Vec<String> = Vec::new();
		let mut excluded_tags: Vec<String> = Vec::new();
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
				FilterValue::Select { id, value } => match id.as_str() {
					"country" => country = value,
					// A tapped tag sends its display name
					"genre" => match tag_filter_id(&value)? {
						Some(tag) => tags.push(tag),
						None => genres.push(value),
					},
					"day" => {
						let (start, end) = value
							.parse()
							.ok()
							.and_then(|days_ago| day_range(days_ago, current_date()))
							.ok_or(error!("Unknown update day"))?;
						search.insert("dateRangeStart".into(), start.into());
						search.insert("dateRangeEnd".into(), end.into());
					}
					"season" => {
						search.insert("season".into(), value.into());
						// The site's season picker always carries a year, the current one by default
						search
							.entry("year")
							.or_insert_with(|| season_and_year(current_date()).1.into());
					}
					"chapters" => {
						// Ranges read `11-50`; the last, `2001`, has no end
						let (start, end) = value.split_once('-').unwrap_or((&value, ""));
						for (key, bound) in [("epRangeStart", start), ("epRangeEnd", end)] {
							if let Ok(bound) = bound.parse::<i32>() {
								search.insert(key.into(), bound.into());
							}
						}
					}
					_ => {}
				},
				FilterValue::Text { id, value } if id == "year" => {
					let year: i32 = value.trim().parse().map_err(|_| error!("Enter a year"))?;
					search.insert("year".into(), year.into());
				}
				// The site's panel only includes; the API also excludes
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} if id == "tags" => {
					tags.extend(included);
					excluded_tags.extend(excluded);
				}
				FilterValue::MultiSelect {
					included, excluded, ..
				} => {
					genres.extend(included);
					excluded_genres.extend(excluded);
				}
				_ => {}
			}
		}
		let (magazines, tags): (Vec<_>, Vec<_>) = tags
			.into_iter()
			.partition(|tag| tag.starts_with("magazine:"));
		match magazines.as_slice() {
			[] => {}
			[magazine] => {
				search.insert("magazine".into(), magazine["magazine:".len()..].into());
			}
			_ => bail!("The site searches one magazine at a time"),
		}
		if excluded_tags.iter().any(|tag| tag.starts_with("magazine:")) {
			bail!("Magazines can only be included");
		}
		for (field, values) in [
			("genres", genres),
			("tags", tags),
			("excludeGenres", excluded_genres),
			("excludeTags", excluded_tags),
		] {
			if !values.is_empty() {
				search.insert(field.into(), values.into());
			}
		}
		let data: SearchData =
			parse(browse_request(Value::Object(search), page, &country)?.send()?)?;
		Ok(MangaPageResult {
			has_next_page: has_next_page(page, data.mangas.page_info.total),
			entries: data
				.mangas
				.edges
				.into_iter()
				.filter_map(|card| Manga::try_from(card).ok())
				.collect(),
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
			let details = data
				.manga
				.ok_or(error!("Series not found"))?
				.into_details(&manga.key)?;
			manga.copy_from(details);
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
		Ok(webview::page_urls(&manga.key, &chapter.key)?
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

impl DynamicFilters for Mkissa {
	// The site's Tags section offers whichever tags are trending
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let data: TagsData = fetch(
			TAGS_QUERY,
			json!({ "search": { "queryType": "Home", "format": "manga" }, "limit": 16 }),
		)?;
		let (ids, options) = data
			.query_tags
			.edges
			.into_iter()
			.map(|tag| (tag.filter_id().into(), tag.name.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "tags".into(),
				title: Some("Tags".into()),
				options,
				ids: Some(ids),
				can_exclude: true,
				uses_tag_style: true,
				hide_from_header: Some(true),
				..Default::default()
			}
			.into(),
		])
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
	DynamicFilters,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
