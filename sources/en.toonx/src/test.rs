#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue,
	Listing, ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;

const SERIES: &str = "dismissed";

type Tested = Toonz<Toonx>;

// The runner keeps a value that is set to `Null`, so each default is written out.
fn excluded(genres: &[&str]) {
	defaults_set(
		"excludedGenres",
		DefaultValue::StringArray(genres.iter().map(|v| v.to_string()).collect()),
	);
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	Tested::new()
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn keys(result: &MangaPageResult) -> Vec<String> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.clone())
		.collect()
}

fn multi(id: &str, included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|v| v.to_string()).collect(),
		excluded: excluded.iter().map(|v| v.to_string()).collect(),
	}
}

fn sort(index: i32) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending: false,
	}
}

fn manga(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

#[aidoku_test]
fn review_home_rows_keep_the_site_order() {
	let titles: Vec<_> = Toonx
		.params()
		.home_rows
		.iter()
		.map(|(title, _)| *title)
		.collect();
	assert_eq!(
		titles,
		[
			"Top Rated This Month",
			"New Titles",
			"Recently Updated",
			"Trending",
			"Popular Updates",
		]
	);
}

#[aidoku_test]
fn browse_pages_and_sorts() {
	excluded(&[]);
	let first = search(None, Vec::new());
	assert_eq!(first.entries.len(), 24);
	assert!(first.has_next_page);
	assert!(
		first
			.entries
			.iter()
			.all(|manga| manga.content_rating == ContentRating::NSFW)
	);
	let second = Tested::new()
		.get_search_manga_list(None, 2, Vec::new())
		.unwrap();
	assert_ne!(keys(&first), keys(&second));
	let mut seen = Vec::new();
	for index in 0..4 {
		let page = keys(&search(None, vec![sort(index)]));
		assert!(!page.is_empty(), "sort {index} is empty");
		assert!(!seen.contains(&page), "sort {index} repeats another sort");
		seen.push(page);
	}
}

#[aidoku_test]
fn search_text_author_and_filters() {
	excluded(&[]);
	assert!(
		keys(&search(Some("dismissed"), Vec::new())).contains(&SERIES.into()),
		"text search"
	);
	assert!(search(Some("zzzxqvqzzk"), Vec::new()).entries.is_empty());
	let author = search(
		None,
		vec![FilterValue::Text {
			id: "author".into(),
			value: "Rose Blue 3D".into(),
		}],
	);
	assert!(keys(&author).contains(&SERIES.into()));

	let all = keys(&search(None, Vec::new()));
	for filter in [
		FilterValue::Select {
			id: "status".into(),
			value: "ongoing".into(),
		},
		multi("genre", &["x-ray"], &[]),
		multi("genre", &[], &["3d"]),
	] {
		let narrowed = search(None, vec![filter]);
		assert!(!narrowed.entries.is_empty());
		assert_ne!(keys(&narrowed), all);
	}
}

#[aidoku_test]
fn excluded_genres_apply_and_yield_to_search() {
	excluded(&["3d"]);
	assert!(!keys(&search(None, Vec::new())).contains(&SERIES.into()));
	let included = search(None, vec![multi("genre", &["3d"], &[])]);
	assert!(
		keys(&included).contains(&SERIES.into()),
		"an included tag must override its exclusion"
	);
	excluded(&[]);
}

#[aidoku_test]
fn home_loads_every_row() {
	excluded(&[]);
	let layout = Tested::new().get_home().unwrap();
	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	let expected: Vec<_> = Toonx
		.params()
		.home_rows
		.iter()
		.map(|(title, _)| *title)
		.collect();
	assert_eq!(titles, expected);
	for component in &layout.components {
		match &component.value {
			HomeComponentValue::BigScroller { entries, .. } => {
				assert!(entries.iter().all(|manga| manga.description.is_some()))
			}
			HomeComponentValue::MangaChapterList { entries, .. } => {
				assert!(
					!entries.is_empty()
						&& entries
							.iter()
							.all(|entry| entry.chapter.chapter_number.is_some())
				);
			}
			HomeComponentValue::Scroller { entries, .. } => assert!(!entries.is_empty()),
			_ => panic!("unexpected component"),
		}
	}
}

#[aidoku_test]
fn listing_is_the_latest_sort() {
	excluded(&[]);
	let listing = Tested::new()
		.get_manga_list(
			Listing {
				id: "latest".into(),
				name: "Latest Updates".into(),
				kind: ListingKind::Default,
			},
			1,
		)
		.unwrap();
	let latest = keys(&search(None, Vec::new()));
	let shared = keys(&listing)
		.iter()
		.filter(|key| latest.contains(key))
		.count();
	assert!(shared >= 20, "only {shared} shared");
}

#[aidoku_test]
fn deep_links_and_pasted_links() {
	let source = Tested::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	for url in [
		"https://toonx.to/adult/dismissed",
		"https://toonx.to/comic/dismissed",
	] {
		assert!(matches!(link(url), Some(DeepLinkResult::Manga { key }) if key == SERIES));
	}
	assert!(matches!(
		link("https://toonx.to/adult/dismissed/chapter/1"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == SERIES && key == "1"
	));
	assert!(link("https://toonx.to/comics?sort=latest").is_none());

	excluded(&[]);
	let pasted = search(Some("https://toonx.to/adult/dismissed"), Vec::new());
	assert_eq!(keys(&pasted), [SERIES]);
	assert_eq!(pasted.entries[0].title, "Dismissed");
	assert!(pasted.entries[0].description.is_none());
}

#[aidoku_test]
fn details_chapters_and_pages() {
	let source = Tested::new();
	let details = source.get_manga_update(manga(SERIES), true, true).unwrap();
	assert_eq!(details.title, "Dismissed");
	assert_eq!(details.authors, Some(vec!["Rose Blue 3D".into()]));
	assert_eq!(details.status, MangaStatus::Completed);
	assert_eq!(details.viewer, Viewer::LeftToRight);
	assert_eq!(details.content_rating, ContentRating::NSFW);
	assert!(details.tags.unwrap().contains(&"3D".into()));
	let chapters = details.chapters.unwrap();
	assert_eq!(chapters.len(), 11);

	let only = source.get_manga_update(manga(SERIES), false, true).unwrap();
	assert!(
		only.title.is_empty(),
		"a chapters-only update fetched details"
	);
	assert_eq!(only.chapters.unwrap().len(), chapters.len());

	let pages = source
		.get_page_list(
			manga(SERIES),
			Chapter {
				key: "1".into(),
				..Default::default()
			},
		)
		.unwrap();
	assert!(pages.len() > 50);
}

#[aidoku_test]
fn unknown_series_fails() {
	assert!(
		Tested::new()
			.get_manga_update(manga("zzzxqv-no-such-series"), true, true)
			.is_err()
	);
}

#[aidoku_test]
fn live_genres_come_from_the_site() {
	use aidoku::{DynamicFilters, DynamicSettings, FilterKind, SettingValue};
	let source = Tested::new();
	let filters = source.get_dynamic_filters().unwrap();
	assert_eq!(filters[0].id, "genre");
	let FilterKind::MultiSelect {
		is_genre: true,
		can_exclude: true,
		options,
		ids: Some(ids),
		..
	} = &filters[0].kind
	else {
		panic!("the genre filter should be an excludable genre list with slugs");
	};
	assert_eq!(options.len(), ids.len());
	assert!(ids.iter().any(|id| id == "x-ray"));
	let settings = source.get_dynamic_settings().unwrap();
	let SettingValue::Group { items, .. } = &settings[0].value else {
		panic!("excluded genres should be a settings group");
	};
	assert_eq!(items[0].key, "excludedGenres");
	let SettingValue::MultiSelect { values, .. } = &items[0].value else {
		panic!("excluded genres should be a multi-select");
	};
	assert_eq!(values, ids);
}
