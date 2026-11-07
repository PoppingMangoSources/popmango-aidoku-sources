#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue, Home,
	HomeComponent, HomeComponentValue, HomeLayout, Link, Listing, ListingProvider, Manga,
	MangaPageResult, MultiSelectFilter, Page, PageContent, Result, Source,
	alloc::{String, Vec, vec},
	helpers::string::PlainText,
	imports::net::Request,
	prelude::*,
};

mod helpers;
mod models;
mod settings;
#[cfg(test)]
mod test;

use helpers::{NON_GENRES, SearchParams, browse, get, parse};
use models::{ChapterResponse, GenreList, Novel, NovelDetails, NovelList};

const BASE_URL: &str = "https://novelarchive.st";
const API_URL: &str = "https://novelarchive.st/api/novels";
const PAGE_SIZE: i32 = 24;
// The fixed feeds return at most 50, whatever the limit.
const FEED_SIZE: i32 = 50;

struct NovelArchive;

impl Source for NovelArchive {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut params = SearchParams {
			query,
			hide_adult: settings::hide_adult(),
			..Default::default()
		};
		for filter in filters {
			match filter {
				// Recently updated is the site's default and isn't sent.
				FilterValue::Sort { index, .. } => {
					params.sort = match index {
						1 => Some("popular"),
						2 => Some("rating"),
						3 => Some("chapters"),
						_ => None,
					}
				}
				FilterValue::Select { id, value } => match id.as_str() {
					"status" => params.status = (value != "all").then_some(value),
					"genre_match" => params.any_genre = value == "any",
					// A tapped tag carries the genre's label.
					"genre" => params.genres = vec![value.to_lowercase()],
					_ => {}
				},
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} if id == "genre" => {
					params.genres = included;
					params.hidden_genres = excluded;
				}
				// Ticked hides adult novels and excluded shows them, overriding the setting.
				FilterValue::Check { id, value } if id == "hide_adult" => {
					params.hide_adult = value == 1
				}
				_ => {}
			}
		}
		browse(params, page)
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let novel = get::<NovelDetails>(&format!("{API_URL}/{}", manga.key))?.novel;
		if needs_chapters {
			manga.chapters = Some(novel.chapters());
		}
		if needs_details {
			let mut details = novel.into_details();
			// The details page renders Markdown; the big scroller shows the text as it is.
			details.description = details.description.map(|text| text.escape_markdown());
			manga.copy_from(details);
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let content = get::<ChapterResponse>(&format!(
			"{API_URL}/{}/chapters/{}?prefetch=1",
			manga.key, chapter.key
		))?
		.chapter
		.content
		.unwrap_or_default();
		let text = content.trim();
		if text.is_empty() {
			bail!("This chapter has no text on Novel Archive yet");
		}
		Ok(vec![Page {
			content: PageContent::text(text.escape_markdown()),
			..Default::default()
		}])
	}
}

impl Home for NovelArchive {
	fn get_home(&self) -> Result<HomeLayout> {
		let hide_adult = settings::hide_adult();
		let sorted = |sort| {
			Request::get(
				SearchParams {
					sort,
					hide_adult,
					..Default::default()
				}
				.url(1),
			)
		};
		let feeds: Vec<Result<Vec<Novel>>> = Request::send_all([
			Request::get(format!("{API_URL}/trending?limit={FEED_SIZE}"))?,
			sorted(Some("popular"))?,
			Request::get(format!("{API_URL}/recently-updated?limit={FEED_SIZE}"))?,
			sorted(Some("rating"))?,
			Request::get(format!("{API_URL}/editors-choice?limit={FEED_SIZE}"))?,
			sorted(None)?,
		])
		.into_iter()
		.map(|response| Ok(parse::<NovelList>(response?)?.novels))
		.collect();
		// A feed that fails drops out; the home only fails when none loaded.
		if feeds.iter().all(Result::is_err) {
			return Err(feeds
				.into_iter()
				.find_map(Result::err)
				.unwrap_or_else(|| error!("Novel Archive's home didn't load")));
		}
		let mut feeds = feeds
			.into_iter()
			.map(|feed| feed.ok().filter(|novels| !novels.is_empty()));

		let cards = |novels: Vec<Novel>| -> Vec<Link> {
			novels
				.into_iter()
				.map(|novel| Manga::from(novel).into())
				.collect()
		};
		let listing = |id: &str, name: &str| {
			Some(Listing {
				id: id.into(),
				name: name.into(),
				..Default::default()
			})
		};
		// Array elements are evaluated in order, so each section takes its own feed.
		let sections = [
			(
				"Trending",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::BigScroller {
						entries: novels.into_iter().map(Novel::into_details).collect(),
						auto_scroll_interval: Some(8.0),
					}),
			),
			(
				"Popular",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::Scroller {
						entries: cards(novels),
						listing: listing("popular", "Popular"),
					}),
			),
			(
				"Recently Updated",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::MangaChapterList {
						page_size: Some(5),
						entries: novels.into_iter().filter_map(Novel::into_latest).collect(),
						listing: None,
					}),
			),
			(
				"Top Rated",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::Scroller {
						entries: cards(novels),
						listing: listing("rating", "Top Rated"),
					}),
			),
			// The feed carries each pick's description and genres.
			(
				"Editor's Choice",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::BigScroller {
						entries: novels.into_iter().map(Novel::into_details).collect(),
						auto_scroll_interval: None,
					}),
			),
			(
				"New in the Archive",
				feeds
					.next()
					.flatten()
					.map(|novels| HomeComponentValue::Scroller {
						entries: cards(novels),
						listing: listing("recent", "New in the Archive"),
					}),
			),
		];
		Ok(HomeLayout {
			components: sections
				.into_iter()
				.filter_map(|(title, value)| {
					Some(HomeComponent {
						title: Some(title.into()),
						subtitle: None,
						value: value?,
					})
				})
				.collect(),
		})
	}
}

impl ListingProvider for NovelArchive {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let sort = match listing.id.as_str() {
			"popular" => Some("popular"),
			"rating" => Some("rating"),
			"recent" => None,
			_ => bail!("Unknown listing"),
		};
		browse(
			SearchParams {
				sort,
				hide_adult: settings::hide_adult(),
				..Default::default()
			},
			page,
		)
	}
}

impl DynamicFilters for NovelArchive {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let (options, ids) = get::<GenreList>(&format!("{API_URL}/genres"))?
			.genres
			.into_iter()
			.filter(|genre| !NON_GENRES.contains(&genre.value.to_lowercase().as_str()))
			.map(|genre| (genre.label.into(), genre.value.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genre".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: true,
				uses_tag_style: true,
				options,
				ids: Some(ids),
				..Default::default()
			}
			.into(),
		])
	}
}

impl DeepLinkHandler for NovelArchive {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(rest) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let rest = rest.split('#').next().unwrap_or_default();
		let (path, query) = rest.split_once('?').unwrap_or((rest, ""));
		let param = |name: &str| {
			query
				.split('&')
				.find_map(|pair| pair.strip_prefix(name)?.strip_prefix('='))
				.filter(|value| !value.is_empty())
		};
		Ok(match path {
			"/novel" => param("id").map(|id| DeepLinkResult::Manga { key: id.into() }),
			"/reader" => param("novel")
				.zip(param("chapter"))
				.map(|(novel, chapter)| DeepLinkResult::Chapter {
					manga_key: novel.into(),
					key: chapter.into(),
				}),
			_ => None,
		})
	}
}

register_source!(
	NovelArchive,
	Home,
	ListingProvider,
	DynamicFilters,
	DeepLinkHandler
);
