use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Manga,
	PageContent, Source,
	alloc::{Vec, format, vec},
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_static_filter_capabilities() {
	let filters: json::Value = json::from_str(FILTERS_JSON).unwrap();
	assert_eq!(filters[0]["type"], "sort");
	assert_eq!(filters[0]["options"].as_array().unwrap().len(), SORT_COUNT);
	for filter in filters.as_array().unwrap() {
		if filter["type"] == "multi-select" {
			assert_eq!(
				filter["canExclude"],
				filter["id"] == "genreIds" && GENRE_EXCLUSION_SUPPORTED
			);
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
	for index in 0..SORT_COUNT as i32 {
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
			.any(|component| component.title.as_deref() == Some(LATEST_TITLE))
	);
	for latest in layout.components.iter().filter(|component| {
		matches!(
			component.title.as_deref(),
			Some("Latest Releases" | "Latest Updates")
		)
	}) {
		let HomeComponentValue::MangaChapterList {
			entries, page_size, ..
		} = &latest.value
		else {
			panic!("Expected latest chapter rows");
		};
		assert_eq!(*page_size, Some(5));
		assert!(!entries.is_empty());
		assert!(
			entries
				.iter()
				.all(|entry| entry.chapter.chapter_number.is_some())
		);
		assert!(
			entries
				.iter()
				.any(|entry| entry.chapter.date_uploaded.is_some())
		);
	}
	let new_title = if BASE_URL.contains("eternalmangas") {
		"Nuevos lanzamientos"
	} else {
		"New Releases"
	};
	assert!(
		layout
			.components
			.iter()
			.any(|c| c.title.as_deref() == Some(new_title))
	);
	assert!(layout.components.iter().any(|c| matches!(
		c.value,
		HomeComponentValue::BigScroller { .. } | HomeComponentValue::ImageScroller { .. }
	)));
	let novels = Target::new()
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "seriesType".into(),
				value: "NOVEL".into(),
			}],
		)
		.unwrap();
	let novel_title = if BASE_URL.contains("eternalmangas") {
		"Novelas"
	} else {
		"Novels"
	};
	assert_eq!(
		layout
			.components
			.iter()
			.any(|c| c.title.as_deref() == Some(novel_title)),
		!novels.entries.is_empty()
	);
}

#[aidoku_test]
fn live_novel_reader() {
	let source = Target::new();
	let novels = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "seriesType".into(),
				value: "NOVEL".into(),
			}],
		)
		.unwrap();
	let Some(novel) = novels.entries.into_iter().next() else {
		aidoku::println!("The current catalogue has no novels");
		return;
	};
	let novel = source.get_manga_update(novel, true, true).unwrap();
	let Some(chapter) = novel.chapters.unwrap().into_iter().find(|c| !c.locked) else {
		aidoku::println!("The sampled novel has no accessible chapters");
		return;
	};
	let pages = source
		.get_page_list(
			Manga {
				key: novel.key,
				..Default::default()
			},
			Chapter {
				key: chapter.key,
				..Default::default()
			},
		)
		.unwrap();
	assert!(matches!(&pages[0].content, PageContent::Text(text) if !text.trim().is_empty()));
}
