use super::*;
use aidoku::{Chapter, DeepLinkHandler, Home, Manga, alloc::Vec};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_deep_links_keep_key_only_reader_routes() {
	let source = VineTheme::<DivaScans>::new();
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
	let source = VineTheme::<DivaScans>::new();
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
	let home = VineTheme::<DivaScans>::new().get_home().unwrap();
	assert!(home.components.len() >= 4, "{:?}", home.components);
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
	let pages = VineTheme::<DivaScans>::new()
		.get_page_list(
			Manga {
				key: "novel/maid-2".into(),
				..Default::default()
			},
			Chapter {
				key: "20".into(),
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
fn review_home_hero_order_and_page_sizes() {
	let flight = r#"1:{"initialSlides":[{"slug":"popular","title":"Popular","coverImage":"/cover.webp"}],"all":[{"slug":"title","title":"Title","coverImage":"/cover.webp","chapters":[{"number":1}]},{"slug":"title","title":"Title","coverImage":"/cover.webp","type":"WEB_NOVEL","chapters":[{"number":1}]}]}"#;
	let mut body = format!("<html><body><script>self.__next_f.push([1,{flight:?}])</script>");
	for (title, marker, kind) in [
		("New Series", "new", "comic"),
		("Latest Novel Updates", "latest", "novel"),
		("Latest Comic Updates", "latest", "comic"),
		("Popular Today", "popular", "comic"),
		("Top 10 in All", "most_popular", "comic"),
		("Featured", "featured", "comic"),
	] {
		body.push_str(&format!("<section><h2>{title}</h2><a href='/series/{kind}/title?ref=homepage_{marker}'><img alt='Title' src='/cover.webp'></a></section>"));
	}
	body.push_str("</body></html>");
	let home = DivaScans.parse_home(&DivaScans.params(), &body).unwrap();
	assert_eq!(
		home.components
			.iter()
			.map(|c| c.title.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"Featured",
			"Popular Today",
			"Latest Comic Updates",
			"Latest Novel Updates",
			"New Series",
			"Top 10 in All"
		]
	);
	assert!(
		matches!(&home.components[0].value, aidoku::HomeComponentValue::BigScroller { entries, .. } if entries[0].key == "title")
	);
	for component in home.components {
		match component.value {
			aidoku::HomeComponentValue::MangaChapterList { page_size, .. } => {
				assert_eq!(page_size, Some(4))
			}
			aidoku::HomeComponentValue::MangaList { page_size, .. } => {
				assert_eq!(page_size, Some(3))
			}
			_ => {}
		}
	}
}
