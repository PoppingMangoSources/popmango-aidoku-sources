#![expect(clippy::unwrap_used)]

use super::{
	Cocomic,
	helpers::{BASE_URL, clean_chapter_title, is_valid_genre, listing, listing_date},
};
use aidoku::{
	DeepLinkHandler, DynamicFilters, Home, HomeComponentValue, ListingProvider, Manga, Source,
	alloc::{String, Vec, format, vec},
};
use aidoku_test::aidoku_test;
use madara::Madara;

#[aidoku_test]
fn review_date_genre_and_chapter_title_helpers() {
	assert!(listing_date("NEW").is_none());
	assert!(listing_date("5m ago").is_some());
	assert!(listing_date("2 days ago").is_some());
	assert!(is_valid_genre("Romance"));
	assert!(!is_valid_genre("Ch.133"));
	assert!(!is_valid_genre("Ch 134"));
	assert!(!is_valid_genre("Chapter 135"));
	assert!(!is_valid_genre("Publication: Ongoing"));
	assert!(!is_valid_genre("Bato Status: Complete"));
	assert!(clean_chapter_title("Chapter 136", Some(136.0)).is_none());
	assert!(clean_chapter_title("Ch. 136", Some(136.0)).is_none());
	assert_eq!(
		clean_chapter_title("Chapter 136 - The Duel", Some(136.0)).as_deref(),
		Some("The Duel")
	);
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
	if let HomeComponentValue::MangaChapterList { entries, .. } = &home.components[2].value {
		assert!(entries.iter().all(|e| e.chapter.url.is_none()));
	}

	let list_page = source
		.get_manga_list(listing("latest", "Latest Updates"), 1)
		.unwrap();
	assert!(!list_page.entries.is_empty());
	assert!(list_page.has_next_page);

	let search = source
		.get_search_manga_list(Some("jinx".into()), 1, vec![])
		.unwrap();
	assert!(!search.entries.is_empty());

	let filters = source.get_dynamic_filters().unwrap();
	assert_eq!(filters.len(), 1);

	let trainer = Manga {
		key: "/manga/genius-martial-arts-trainer/".into(),
		title: String::new(),
		..Default::default()
	};
	let details = source.get_manga_update(trainer, true, true).unwrap();
	assert!(!details.title.is_empty());
	assert!(details.cover.is_some());
	let tags = details.tags.as_ref().unwrap();
	assert!(!tags.is_empty());
	assert!(
		tags.iter().all(|t| !t.starts_with("Ch.")),
		"chapter tags leaked into details: {tags:?}"
	);
	let chapters = details.chapters.as_ref().unwrap();
	assert!(!chapters.is_empty());
	assert!(
		chapters.iter().take(5).all(|c| {
			c.title
				.as_deref()
				.is_none_or(|t| !t.to_ascii_lowercase().starts_with("chapter "))
		}),
		"chapter title repeated chapter number"
	);

	let pages = source
		.get_page_list(details.clone(), chapters[0].clone())
		.unwrap();
	assert!(!pages.is_empty());

	let deep = source
		.handle_deep_link(format!("{BASE_URL}/manga/genius-martial-arts-trainer/"))
		.unwrap();
	assert!(deep.is_some());
}
