use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Manga, Source,
	alloc::{Vec, format, vec},
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_static_filter_capabilities() {
	let filters: json::Value = json::from_str(FILTERS_JSON).unwrap();
	assert_eq!(filters[0]["type"], "sort");
	assert_eq!(filters[0]["options"].as_array().unwrap().len(), 5);
	for filter in filters.as_array().unwrap() {
		if filter["type"] == "multi-select" {
			assert_eq!(filter["canExclude"], false);
			assert_eq!(
				filter["options"].as_array().unwrap().len(),
				filter["ids"].as_array().unwrap().len()
			);
		}
	}
}

#[aidoku_test]
fn live_search_pagination_sorts_and_clearing() {
	let source = Target::new();
	let first = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(first.has_next_page && !first.entries.is_empty() && !second.entries.is_empty());
	assert_ne!(first.entries[0].key, second.entries[0].key);
	for manga in &first.entries {
		assert!(!manga.key.is_empty() && !manga.title.trim().is_empty());
		assert!(manga.description.is_none() && manga.tags.is_none() && manga.authors.is_none());
	}
	assert!(
		source
			.get_search_manga_list(Some("zzzz_unmatchable_481902".into()), 1, Vec::new())
			.unwrap()
			.entries
			.is_empty()
	);
	let cleared = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genreIds".into(),
				included: Vec::new(),
				excluded: Vec::new(),
			}],
		)
		.unwrap();
	assert_eq!(
		first.entries.iter().map(|m| &m.key).collect::<Vec<_>>(),
		cleared.entries.iter().map(|m| &m.key).collect::<Vec<_>>()
	);
	for index in 0..5 {
		let desc = source
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Sort {
					id: "sort".into(),
					index,
					ascending: false,
				}],
			)
			.unwrap();
		let asc = source
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Sort {
					id: "sort".into(),
					index,
					ascending: true,
				}],
			)
			.unwrap();
		assert!(!asc.entries.is_empty() && !desc.entries.is_empty());
		assert_ne!(asc.entries[0].key, desc.entries[0].key);
	}
	assert!(
		source
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::MultiSelect {
					id: "genreIds".into(),
					included: vec!["2147483647".into()],
					excluded: Vec::new()
				}]
			)
			.unwrap()
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn live_details_chapters_pages_and_deep_links() {
	let source = Target::new();
	let detail = source
		.get_manga_update(
			Manga {
				key: SERIES_SLUG.into(),
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert!(detail.key.parse::<i32>().is_ok() && !detail.title.is_empty());
	assert!(detail.chapters.is_none());
	let chapters = source
		.get_manga_update(
			Manga {
				key: detail.key.clone(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap()
		.chapters
		.unwrap();
	assert!(!chapters.is_empty());
	for chapter in chapters
		.iter()
		.filter(|c| !c.locked)
		.take(1)
		.chain(chapters.iter().rev().filter(|c| !c.locked).take(1))
	{
		let pages = source
			.get_page_list(
				Manga {
					key: detail.key.clone(),
					..Default::default()
				},
				Chapter {
					key: chapter.key.clone(),
					..Default::default()
				},
			)
			.unwrap();
		assert!(!pages.is_empty());
	}
	let combined = source
		.get_manga_update(
			Manga {
				key: detail.key.clone(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(combined.chapters.unwrap().len(), chapters.len());
	let url = format!("{BASE_URL}/series/{SERIES_SLUG}");
	assert_eq!(
		source.handle_deep_link(url.clone()).unwrap(),
		Some(DeepLinkResult::Manga {
			key: detail.key.clone()
		})
	);
	assert_eq!(
		source
			.get_search_manga_list(Some(url), 1, Vec::new())
			.unwrap()
			.entries[0]
			.key,
		detail.key
	);
	let chapter = &chapters[0];
	assert_eq!(
		source
			.handle_deep_link(chapter.url.clone().unwrap())
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: detail.key,
			key: chapter.key.clone()
		})
	);
}

#[aidoku_test]
fn live_home_has_site_rows() {
	let layout = Target::new().get_home().unwrap();
	assert!(
		layout
			.components
			.iter()
			.any(|c| c.title.as_deref() == Some(LATEST_TITLE))
	);
	assert!(layout.components.iter().any(|c| matches!(
		c.value,
		HomeComponentValue::BigScroller { .. } | HomeComponentValue::ImageScroller { .. }
	)));
}
