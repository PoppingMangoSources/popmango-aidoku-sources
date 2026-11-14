use super::*;
use aidoku::{Chapter, DeepLinkHandler, Home, Manga, alloc::Vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_deep_links_keep_key_only_reader_routes() {
	let source = VineTheme::<ValirScans>::new();
	for kind in ["comic", "novel"] {
		let result = source
			.handle_deep_link(format!(
				"{BASE_URL}/series/{kind}/title/chapter/2.5/?ref=home"
			))
			.unwrap()
			.unwrap();
		let expected = if kind == "novel" {
			"novel/title"
		} else {
			"title"
		};
		assert!(
			matches!(result, aidoku::DeepLinkResult::Chapter { manga_key, key } if manga_key == expected && key == "2.5")
		);
	}
}

#[aidoku_test]
fn catalogue_details_chapters_and_pages() {
	let source = VineTheme::<ValirScans>::new();
	let search = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	assert!(!search.entries.is_empty());
	let first = search.entries.into_iter().next().unwrap();
	assert!(first.description.is_none() && first.authors.is_none() && first.tags.is_none());
	let manga = source
		.get_manga_update(
			Manga {
				key: first.key.clone(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert!(!manga.title.is_empty());
	let chapter = manga
		.chapters
		.unwrap()
		.into_iter()
		.find(|c| !c.locked)
		.unwrap();
	let pages = source
		.get_page_list(
			Manga {
				key: first.key,
				..Default::default()
			},
			Chapter {
				key: chapter.key,
				..Default::default()
			},
		)
		.unwrap();
	assert!(!pages.is_empty());
}

#[aidoku_test]
fn home_has_no_duplicate_series_per_row() {
	let home = VineTheme::<ValirScans>::new().get_home().unwrap();
	assert!(home.components.len() >= 5, "{:?}", home.components);
	for component in home.components {
		let keys: Vec<_> = match component.value {
			aidoku::HomeComponentValue::MangaChapterList { entries, .. } => {
				entries.into_iter().map(|entry| entry.manga.key).collect()
			}
			aidoku::HomeComponentValue::Scroller { entries, .. }
			| aidoku::HomeComponentValue::MangaList { entries, .. } => entries
				.into_iter()
				.map(|entry| match entry.value {
					Some(aidoku::LinkValue::Manga(manga)) => manga.key,
					_ => panic!(),
				})
				.collect(),
			aidoku::HomeComponentValue::BigScroller { entries, .. } => {
				entries.into_iter().map(|entry| entry.key).collect()
			}
			_ => continue,
		};
		let unique: aidoku::alloc::collections::BTreeSet<_> = keys.iter().collect();
		assert_eq!(unique.len(), keys.len());
	}
}

#[aidoku_test]
fn novel_chapter_reads_from_keys_only() {
	let pages = VineTheme::<ValirScans>::new()
		.get_page_list(
			Manga {
				key: "novel/post-possession-damage-control".into(),
				..Default::default()
			},
			Chapter {
				key: "194".into(),
				..Default::default()
			},
		)
		.unwrap();
	assert_eq!(pages.len(), 1);
	assert!(
		matches!(&pages[0].content, aidoku::PageContent::Text(text) if text.len() > 500 && text.contains("\n\n") && !text.starts_with('$'))
	);
}

#[aidoku_test]
fn review_home_order_sizes_and_compact_novels() {
	use aidoku::{HomeComponent, HomeComponentValue as Value, HomeLayout, alloc::vec};
	let home = HomeLayout {
		components: [
			"New Series",
			"Latest Novel Updates",
			"Editor's Pick",
			"Top 10 in All",
			"Latest Comic Updates",
			"Popular Today",
		]
		.into_iter()
		.map(|title| HomeComponent {
			title: Some(title.into()),
			value: match title {
				"Latest Novel Updates" | "Latest Comic Updates" => Value::MangaChapterList {
					entries: vec![],
					page_size: Some(5),
					listing: None,
				},
				"Top 10 in All" => Value::MangaList {
					entries: vec![],
					ranking: true,
					page_size: Some(5),
					listing: None,
				},
				"Editor's Pick" => Value::BigScroller {
					entries: vec![Manga {
						key: "pick".into(),
						title: "Pick".into(),
						..Default::default()
					}],
					auto_scroll_interval: None,
				},
				_ => Value::Scroller {
					entries: vec![],
					listing: None,
				},
			},
			..Default::default()
		})
		.collect(),
	};
	let home = home::layout(
		home,
		vec![Manga {
			key: "trending".into(),
			title: "Trending title".into(),
			description: Some("Description".into()),
			..Default::default()
		}],
		vec![Manga {
			key: "novel/title".into(),
			title: "Title".into(),
			description: Some("Details only".into()),
			..Default::default()
		}],
	);
	assert_eq!(
		home.components
			.iter()
			.map(|c| c.title.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"Trending",
			"Popular Today",
			"Latest Comic Updates",
			"Novels",
			"Latest Novel Updates",
			"Editor's Pick",
			"Top 10 in All"
		]
	);
	assert!(
		matches!(&home.components[0].value, Value::BigScroller { entries, .. } if entries[0].key == "trending" && entries[0].description.is_some())
	);
	for index in [2, 4] {
		assert!(matches!(
			&home.components[index].value,
			Value::MangaChapterList {
				page_size: Some(4),
				..
			}
		));
	}
	assert!(
		matches!(&home.components[5].value, Value::BigScroller { entries, .. } if entries[0].key == "pick")
	);
	assert!(matches!(
		&home.components[6].value,
		Value::MangaList {
			page_size: Some(3),
			ranking: true,
			..
		}
	));
	let Value::Scroller { entries, listing } = &home.components[3].value else {
		panic!()
	};
	assert_eq!(listing.as_ref().unwrap().id, "type=Web%20Novel");
	assert!(
		matches!(&entries[0].value, Some(aidoku::LinkValue::Manga(manga)) if manga.description.is_none())
	);
	assert!(
		home::layout(HomeLayout::default(), vec![], vec![])
			.components
			.is_empty()
	);
}

#[aidoku_test]
fn live_home_has_requested_rows() {
	let home = VineTheme::<ValirScans>::new().get_home().unwrap();
	assert_eq!(
		home.components
			.iter()
			.map(|c| c.title.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"Trending",
			"Popular Today",
			"Latest Comic Updates",
			"Novels",
			"Latest Novel Updates",
			"Editor's Pick",
			"Top 10 in All"
		]
	);
	assert!(
		matches!(&home.components[5].value, aidoku::HomeComponentValue::BigScroller { entries, .. } if entries.len() > 1)
	);
}

#[aidoku_test]
fn live_novel_listing_preserves_type_and_pagination() {
	use aidoku::{Listing, ListingProvider};
	let source = VineTheme::<ValirScans>::new();
	let listing = Listing {
		id: "type=Web%20Novel".into(),
		..Default::default()
	};
	let first = source.get_manga_list(listing.clone(), 1).unwrap();
	let second = source.get_manga_list(listing, 2).unwrap();
	assert!(first.has_next_page && !first.entries.is_empty() && !second.entries.is_empty());
	assert!(
		first
			.entries
			.iter()
			.chain(&second.entries)
			.all(|manga| manga.key.starts_with("novel/"))
	);
	assert_ne!(first.entries[0].key, second.entries[0].key);
}
