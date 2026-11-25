#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{DeepLinkHandler, DynamicFilters, Home, ListingProvider, Source};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_date_and_genre_helpers() {
	assert!(listing_date("NEW").is_none());
	assert!(listing_date("5m ago").is_some());
	assert!(listing_date("2 days ago").is_some());
	assert!(is_valid_genre("Romance"));
	assert!(!is_valid_genre("Publication: Ongoing"));
	assert!(!is_valid_genre("Bato Status: Complete"));
}

#[aidoku_test]
fn home_listings_search_details_and_pages() {
	let source = Madara::<Cocomic>::new();

	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Trending",
			"Trending Now",
			"Latest Updates",
			"Popular Manga",
			"New Releases",
		]
	);

	let listing = source
		.get_manga_list(make_listing("latest", "Latest Updates"), 1)
		.unwrap();
	assert!(!listing.entries.is_empty());
	assert!(listing.has_next_page);

	let search = source
		.get_search_manga_list(Some("jinx".into()), 1, vec![])
		.unwrap();
	assert!(!search.entries.is_empty());

	let filters = source.get_dynamic_filters().unwrap();
	assert_eq!(filters.len(), 1);

	let first = Manga {
		key: "/manga/104701-jinx-official/".into(),
		title: String::new(),
		..Default::default()
	};
	let details = source.get_manga_update(first, true, true).unwrap();
	assert!(!details.title.is_empty());
	assert!(details.cover.is_some());
	assert!(details.tags.as_ref().is_some_and(|t| !t.is_empty()));
	let chapters = details.chapters.as_ref().unwrap();
	assert!(!chapters.is_empty());

	let pages = source
		.get_page_list(details.clone(), chapters[0].clone())
		.unwrap();
	assert!(!pages.is_empty());

	let deep = source
		.handle_deep_link(format!("{BASE_URL}/manga/104701-jinx-official/"))
		.unwrap();
	assert!(deep.is_some());
}
