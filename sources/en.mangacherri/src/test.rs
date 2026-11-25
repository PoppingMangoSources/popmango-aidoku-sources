#![expect(clippy::unwrap_used)]

use super::{
	MangaCherri,
	helpers::{BASE_URL, chapter_id_from_href, listing, parse_chapter_label, slug_from_href},
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
		parse_chapter_label("Chapter 500.5 OMAKE"),
		(Some(500.5), Some("OMAKE".into()))
	);
	assert_eq!(parse_chapter_label("Ch. 12"), (Some(12.0), None));
	assert_eq!(parse_chapter_label("12"), (Some(12.0), None));
	assert_eq!(
		slug_from_href("https://mangacherri.com/title/a-beloved-existence"),
		Some("a-beloved-existence".into())
	);
	assert_eq!(
		chapter_id_from_href("https://mangacherri.com/read/49974"),
		Some("49974".into())
	);
}

#[aidoku_test]
fn home_catalogue_details_pages_and_deep_links() {
	let source = MangaCherri::new();

	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Latest Updates",
			"Most Viewed",
			"Stories Still Unfolding",
			"Completed",
			"Recently Added",
		]
	);
	if let HomeComponentValue::MangaChapterList { entries, .. } = &home.components[0].value {
		assert!(entries.iter().all(|e| e.chapter.url.is_none()));
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
				value: "romance".into(),
			}],
		)
		.unwrap();
	assert!(!filtered.entries.is_empty());

	let list_page = source
		.get_manga_list(listing("completed", "Completed"), 1)
		.unwrap();
	assert!(!list_page.entries.is_empty());

	let sample = Manga {
		key: "a-beloved-existence".into(),
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
			.handle_deep_link(format!("{BASE_URL}/title/a-beloved-existence"))
			.unwrap(),
		Some(DeepLinkResult::Manga { key }) if key == "a-beloved-existence"
	));
}
