#![expect(clippy::unwrap_used)]

use super::{
	MangaBerri,
	helpers::{BASE_URL, chapter_from_href, listing, parse_chapter_label, slug_from_href},
};
use aidoku::{
	DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, ListingProvider, Manga,
	Source,
	alloc::{String, Vec, format, vec},
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_chapter_and_link_helpers() {
	assert_eq!(
		parse_chapter_label("Chapter 202 END"),
		(Some(202.0), Some("END".into()))
	);
	assert_eq!(parse_chapter_label("Chapter 12"), (Some(12.0), None));
	assert_eq!(parse_chapter_label("Ch. 12"), (Some(12.0), None));
	assert_eq!(parse_chapter_label("12"), (Some(12.0), None));
	assert_eq!(
		slug_from_href("https://mangaberri.com/title/beyblade-x"),
		Some("beyblade-x".into())
	);
	assert_eq!(
		chapter_from_href("https://mangaberri.com/read/beyblade-x/35548"),
		Some(("beyblade-x".into(), "35548".into()))
	);
}

#[aidoku_test]
fn home_browse_details_pages_and_deep_links() {
	let source = MangaBerri::new();

	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(titles, ["Most Read", "Popular", "Latest Updates"]);
	match &home.components[0].value {
		HomeComponentValue::BigScroller { entries, .. } => {
			assert!(
				entries.len() > 1,
				"Most Read BigScroller must have multiple titles"
			);
			assert!(entries.iter().any(|e| e.tags.is_some()));
		}
		_ => panic!("Most Read must be BigScroller"),
	}

	let browse = source.get_search_manga_list(None, 1, vec![]).unwrap();
	assert!(!browse.entries.is_empty());
	assert!(browse.has_next_page);

	let filtered = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "genre".into(),
				value: "Action".into(),
			}],
		)
		.unwrap();
	assert!(!filtered.entries.is_empty());

	let list_page = source
		.get_manga_list(listing("popular", "Popular"), 1)
		.unwrap();
	assert!(!list_page.entries.is_empty());

	let sample = Manga {
		key: "beyblade-x".into(),
		title: String::new(),
		..Default::default()
	};
	let details = source.get_manga_update(sample, true, true).unwrap();
	assert!(!details.title.is_empty());
	assert!(details.cover.is_some());
	let chapters = details.chapters.as_ref().unwrap();
	assert!(!chapters.is_empty());

	let pages = source
		.get_page_list(details.clone(), chapters[0].clone())
		.unwrap();
	assert!(!pages.is_empty());

	assert!(matches!(
		source
			.handle_deep_link(format!("{BASE_URL}/title/beyblade-x"))
			.unwrap(),
		Some(DeepLinkResult::Manga { key }) if key == "beyblade-x"
	));
	assert!(matches!(
		source
			.handle_deep_link(format!("{BASE_URL}/read/beyblade-x/35548"))
			.unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == "beyblade-x" && key == "35548"
	));
}
