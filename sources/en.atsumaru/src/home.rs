use crate::{
	Atsumaru,
	api::{FEATURED_FIELDS, FEED_LIMIT, Search, feed_url, get, lookup_url, parse, request},
	helpers::{feed_allows, spotlight_genre},
	models::{FeedItem, FeedResponse, SearchDocument, SearchResponse},
	settings,
};
use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Listing, ListingKind,
	MangaPageResult, Result,
	alloc::{String, Vec, format},
	imports::{
		net::{Request, Response},
		std::send_partial_result,
	},
};

#[derive(Clone, Copy)]
enum Layout {
	Featured,
	Chapters,
	Ranked,
	Row,
}

const NOVELS: &str = "novels";
const SPOTLIGHT: &str = "genreSpotlight";

/// Listing id, title, subtitle and layout; the ids double as listing ids.
const SECTIONS: [(&str, &str, Option<&str>, Layout); 14] = [
	(
		"popular",
		"Popular This Week",
		Some("Most read this week"),
		Layout::Featured,
	),
	(
		"recentlyUpdated",
		"Recently Updated",
		Some("Fresh chapters"),
		Layout::Chapters,
	),
	(
		"hotUpdates",
		"Hot Updates",
		Some("Busy right now"),
		Layout::Row,
	),
	("rising", "Rising", Some("Climbing fast"), Layout::Row),
	(
		"topRated",
		"Top Rated",
		Some("Highest scores"),
		Layout::Ranked,
	),
	(SPOTLIGHT, "Spotlight", None, Layout::Row),
	(
		"hotArrivals",
		"Hot Arrivals",
		Some("New and popular"),
		Layout::Row,
	),
	(
		"mostBookmarked",
		"Most Bookmarked This Week",
		Some("Reader favourites"),
		Layout::Ranked,
	),
	(
		"mostTalkedAbout",
		"Most Talked About This Week",
		Some("Most discussed"),
		Layout::Row,
	),
	(
		NOVELS,
		"Most Popular Novels",
		Some("All time"),
		Layout::Ranked,
	),
	(
		"recentlyAdded",
		"Recently Added",
		Some("New titles"),
		Layout::Row,
	),
	("hiddenGems", "Hidden Gems", None, Layout::Row),
	("bingeWorthy", "Binge-Worthy", None, Layout::Row),
	("mostPolarizing", "Most Polarizing", None, Layout::Row),
];

fn title(id: &str, title: &str) -> String {
	if id == SPOTLIGHT {
		format!("{title}: {}", spotlight_genre())
	} else {
		title.into()
	}
}

fn section_request(id: &str, page: i32) -> Result<Option<Request>> {
	if id == NOVELS {
		let search = Search::novels();
		if search.matches_nothing() {
			return Ok(None);
		}
		return request(&search.url(page)).map(Some);
	}
	feed_url(id, page)?.map(|url| request(&url)).transpose()
}

fn feed_items(response: Response) -> Result<(Vec<FeedItem>, bool)> {
	let items = parse::<FeedResponse>(response)?.items;
	// Feeds give no total, so a full page is the only sign of another.
	let has_next_page = items.len() as i32 == FEED_LIMIT;
	let types = settings::content_types();
	let items = items
		.into_iter()
		.filter(|item| feed_allows(item.kind.as_deref(), item.medium.as_deref(), &types))
		.collect();
	Ok((items, has_next_page))
}

pub fn listing_page(id: &str, page: i32) -> Result<MangaPageResult> {
	let label_novels = settings::label_novels();
	if id == NOVELS {
		return Search::novels().page(page, label_novels);
	}
	let Some(request) = section_request(id, page)? else {
		return Ok(MangaPageResult::default());
	};
	let (items, has_next_page) = feed_items(request.send()?)?;
	Ok(MangaPageResult {
		entries: items
			.into_iter()
			.map(|item| item.into_manga(label_novels))
			.collect(),
		has_next_page,
	})
}

/// The feed has no descriptions or authors, so one search request fills them in.
fn featured_documents(items: &[FeedItem]) -> Vec<SearchDocument> {
	let ids = items
		.iter()
		.map(|item| item.id.as_str())
		.collect::<Vec<_>>();
	get::<SearchResponse>(&lookup_url(&ids, FEATURED_FIELDS))
		.map(|response| response.hits.into_iter().map(|hit| hit.document).collect())
		.unwrap_or_default()
}

fn fill(id: &str, name: &str, layout: Layout, response: Response) -> Result<HomeComponentValue> {
	let label_novels = settings::label_novels();
	let listing = Some(Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	});
	if id == NOVELS {
		let response: SearchResponse = parse(response)?;
		return Ok(HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(5),
			entries: response
				.hits
				.into_iter()
				.map(|hit| hit.document.into_link(label_novels))
				.collect(),
			listing,
		});
	}
	let (items, _) = feed_items(response)?;
	Ok(match layout {
		Layout::Featured => {
			let mut documents = featured_documents(&items);
			HomeComponentValue::BigScroller {
				entries: items
					.into_iter()
					.map(|item| {
						let document = documents
							.iter()
							.position(|document| document.id == item.id)
							.map(|index| documents.swap_remove(index));
						item.into_featured(label_novels, document)
					})
					.collect(),
				auto_scroll_interval: Some(8.0),
			}
		}
		Layout::Chapters => HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries: items
				.into_iter()
				.map(|item| item.into_manga_with_chapter(label_novels))
				.collect(),
			listing,
		},
		Layout::Ranked => HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(5),
			entries: items
				.into_iter()
				.map(|item| item.into_link(label_novels))
				.collect(),
			listing,
		},
		Layout::Row => HomeComponentValue::Scroller {
			entries: items
				.into_iter()
				.map(|item| item.into_link(label_novels))
				.collect(),
			listing,
		},
	})
}

fn empty(layout: Layout) -> HomeComponentValue {
	match layout {
		Layout::Featured => HomeComponentValue::empty_big_scroller(),
		Layout::Chapters => HomeComponentValue::empty_manga_chapter_list(),
		Layout::Ranked => HomeComponentValue::empty_manga_list(),
		Layout::Row => HomeComponentValue::empty_scroller(),
	}
}

impl Home for Atsumaru {
	fn get_home(&self) -> Result<HomeLayout> {
		let shown = settings::home_sections();
		let mut sections = Vec::new();
		let mut requests = Vec::new();
		for (id, name, subtitle, layout) in SECTIONS {
			if shown
				.as_ref()
				.is_some_and(|shown| !shown.iter().any(|value| value == id))
			{
				continue;
			}
			if let Some(request) = section_request(id, 1)? {
				sections.push((id, title(id, name), subtitle, layout));
				requests.push(request);
			}
		}

		// The layout goes out first so the page shows while the sections load.
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: sections
				.iter()
				.map(|(_, name, subtitle, layout)| HomeComponent {
					title: Some(name.clone()),
					subtitle: subtitle.map(Into::into),
					value: empty(*layout),
				})
				.collect(),
		}));

		for ((id, name, subtitle, layout), response) in
			sections.into_iter().zip(Request::send_all(requests))
		{
			let Ok(value) = response
				.map_err(Into::into)
				.and_then(|response| fill(id, &name, layout, response))
			else {
				continue;
			};
			send_partial_result(&HomePartialResult::Component(HomeComponent {
				title: Some(name),
				subtitle: subtitle.map(Into::into),
				value,
			}));
		}

		Ok(HomeLayout::default())
	}
}
