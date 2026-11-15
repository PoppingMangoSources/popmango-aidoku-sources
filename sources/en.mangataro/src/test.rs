use super::*;
use aidoku::{Home, HomeComponentValue, Manga, Source, alloc::Vec};
use aidoku_test::aidoku_test;

type SourceImpl = MangaTaro<MangaTaroSource>;

#[aidoku_test]
fn search_pagination_and_impossible_query() {
	let source = SourceImpl::new();
	let first = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(!first.entries.is_empty());
	assert_ne!(first.entries[0].key, second.entries[0].key);
	assert!(
		first
			.entries
			.iter()
			.all(|m| m.authors.is_none() && m.description.is_none() && m.tags.is_none())
	);
	let absent = source
		.get_search_manga_list(Some("zzzzzznoexistingtitle987654321".into()), 1, Vec::new())
		.unwrap();
	assert!(absent.entries.is_empty());
}

#[aidoku_test]
fn details_chapters_and_key_only_reader() {
	let source = SourceImpl::new();
	let manga = source
		.get_manga_update(
			Manga {
				key: "terminally-ill-genius-dark-knight".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.description.is_some());
	let chapter = manga.chapters.as_ref().unwrap().first().unwrap();
	assert!(chapter.chapter_number.is_some());
	let pages = source
		.get_page_list(
			Manga {
				key: manga.key.clone(),
				..Default::default()
			},
			aidoku::Chapter {
				key: chapter.key.clone(),
				..Default::default()
			},
		)
		.unwrap();
	assert!(!pages.is_empty());
	let chapters_only = source
		.get_manga_update(
			Manga {
				key: manga.key,
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert!(!chapters_only.chapters.unwrap().is_empty());
}

#[aidoku_test]
fn home_has_ranking_and_dated_updates() {
	let home = SourceImpl::new().get_home().unwrap();
	assert!(home.components.len() >= 7);
	assert!(home.components.iter().any(|c| matches!(
		&c.value,
		HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(3),
			..
		}
	)));
	assert!(home.components.iter().any(|c| matches!(&c.value, HomeComponentValue::MangaChapterList { page_size: Some(4), entries, .. } if entries.iter().any(|m| m.chapter.chapter_number.is_some() && m.chapter.date_uploaded.is_some()))));
}

#[aidoku_test]
fn novel_key_only_reading_and_errors() {
	let source = SourceImpl::new();
	let novel = source
		.get_manga_update(
			Manga {
				key: "reborn-as-the-h-ng-legacy".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert!(novel.title.ends_with("(Novel)"));
	let chapter = novel.chapters.unwrap().into_iter().next().unwrap();
	let pages = source
		.get_page_list(
			Manga::default(),
			aidoku::Chapter {
				key: chapter.key,
				..Default::default()
			},
		)
		.unwrap();
	assert!(pages.iter().any(
		|page| matches!(&page.content, aidoku::PageContent::Text(text) if text.contains("\n\n"))
	));
	assert!(
		source
			.get_page_list(
				Manga::default(),
				aidoku::Chapter {
					key: "invalid".into(),
					..Default::default()
				}
			)
			.is_err()
	);
}

#[aidoku_test]
fn filter_capabilities_and_type_override() {
	use aidoku::{
		DynamicFilters, FilterKind, FilterValue,
		alloc::vec,
		imports::defaults::{DefaultValue, defaults_set},
	};
	let source = SourceImpl::new();
	let filters = source.get_dynamic_filters().unwrap();
	for id in ["type", "status", "year", "genre"] {
		assert!(filters.iter().any(|f| f.id == id));
	}
	assert!(filters.iter().any(|f| f.id == "genre"
		&& matches!(
			f.kind,
			FilterKind::MultiSelect {
				is_genre: true,
				can_exclude: true,
				uses_tag_style: true,
				..
			}
		)));
	for id in ["status", "year"] {
		assert!(
			filters
				.iter()
				.any(|f| f.id == id && matches!(f.kind, FilterKind::Select { .. }))
		);
	}
	defaults_set(
		"excludedTypes",
		DefaultValue::StringArray(vec!["Novel".into()]),
	);
	let novels = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "type".into(),
				included: vec!["Novel".into()],
				excluded: Vec::new(),
			}],
		)
		.unwrap();
	assert!(!novels.entries.is_empty());
	assert!(novels.entries.iter().all(|m| m.title.ends_with("(Novel)")));
	let none = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genre".into(),
				included: vec!["999999999".into()],
				excluded: Vec::new(),
			}],
		)
		.unwrap();
	assert!(none.entries.is_empty());
	defaults_set("excludedTypes", DefaultValue::StringArray(Vec::new()));
}

#[aidoku_test]
fn each_sort_direction_decodes_and_narrows() {
	use aidoku::{FilterValue, alloc::vec};
	let source = SourceImpl::new();
	for index in 0..4 {
		for ascending in [false, true] {
			let page = source
				.get_search_manga_list(
					None,
					1,
					vec![FilterValue::Sort {
						id: "sort".into(),
						index,
						ascending,
					}],
				)
				.unwrap();
			assert!(
				!page.entries.is_empty(),
				"sort {index}, ascending {ascending}"
			);
			assert!(
				page.entries
					.iter()
					.all(|m| !m.key.is_empty() && !m.title.trim().is_empty())
			);
		}
	}
}

#[aidoku_test]
fn deep_links_unknown_key_and_each_filter() {
	use aidoku::{
		DeepLinkHandler, DeepLinkResult, DynamicFilters, FilterKind, FilterValue, MangaStatus,
		alloc::{format, string::String, vec},
	};
	let source = SourceImpl::new();
	let key = "terminally-ill-genius-dark-knight";
	let unknown = Manga {
		key: "zzzz-not-a-series-987654".into(),
		..Default::default()
	};
	assert!(source.get_manga_update(unknown, true, false).is_err());
	let series_url = format!("{BASE_URL}/manga/{key}");
	assert!(matches!(
		source.handle_deep_link(series_url.clone()).unwrap(),
		Some(DeepLinkResult::Manga { key: found }) if found == key
	));
	let pasted = source
		.get_search_manga_list(Some(series_url), 1, Vec::new())
		.unwrap();
	assert_eq!(pasted.entries.len(), 1);
	assert_eq!(pasted.entries[0].key, key);
	let manga = Manga {
		key: key.into(),
		..Default::default()
	};
	let chapter = source
		.get_manga_update(manga, false, true)
		.unwrap()
		.chapters
		.unwrap()
		.remove(0);
	assert!(matches!(
		source.handle_deep_link(chapter.url.clone().unwrap()).unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, key: found })
			if manga_key == key && found == chapter.key
	));

	let filters = source.get_dynamic_filters().unwrap();
	assert_eq!(filters.last().unwrap().id, "genreMatchMode");
	assert!(
		filters
			.iter()
			.any(|f| f.id == "genre" && f.title.as_deref() == Some("Tags"))
	);
	let kind = |id: &str| &filters.iter().find(|f| f.id == id).unwrap().kind;
	let FilterKind::MultiSelect { options, ids, .. } = kind("genre") else {
		panic!("genre");
	};
	let index = options.iter().position(|o| o == "Action").unwrap();
	let action = ids.as_ref().unwrap()[index].clone().into_owned();
	let FilterKind::Select { ids: years, .. } = kind("year") else {
		panic!("year");
	};
	let years = years.as_ref().unwrap();
	assert!(years[0].is_empty());
	assert!(
		matches!(kind("status"), FilterKind::Select { ids: Some(ids), .. } if ids[0].is_empty())
	);
	let select = |id: &str, value: &str| {
		source
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Select {
					id: id.into(),
					value: value.into(),
				}],
			)
			.unwrap()
			.entries
	};
	let search = |id: &str, included: Vec<String>, excluded: Vec<String>| {
		source
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::MultiSelect {
					id: id.into(),
					included,
					excluded,
				}],
			)
			.unwrap()
			.entries
	};
	let keys = |entries: &[Manga]| entries.iter().map(|m| m.key.clone()).collect::<Vec<_>>();
	let unfiltered = keys(
		&source
			.get_search_manga_list(None, 1, Vec::new())
			.unwrap()
			.entries,
	);

	let completed = select("status", "Completed");
	assert!(!completed.is_empty());
	assert!(completed.iter().all(|m| m.status == MangaStatus::Completed));

	let year = select("year", &years[1]);
	assert!(!year.is_empty());
	assert_ne!(keys(&year), unfiltered);

	let with_action = keys(&search("genre", vec![action.clone()], Vec::new()));
	let without_action = keys(&search("genre", Vec::new(), vec![action]));
	assert!(!with_action.is_empty() && !without_action.is_empty());
	assert!(without_action.iter().all(|k| !with_action.contains(k)));

	assert_eq!(keys(&select("genre", "Action")), with_action);

	let no_novels = search("type", Vec::new(), vec!["Novel".into()]);
	assert!(!no_novels.is_empty());
	assert!(no_novels.iter().all(|m| !m.title.ends_with("(Novel)")));
}
