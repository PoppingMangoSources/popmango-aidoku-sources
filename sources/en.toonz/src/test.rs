#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue,
	Listing, ListingKind, ListingProvider, Manga, MangaPageResult,
	alloc::{String, Vec, string::ToString, vec},
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;

const SERIES: &str = "solo-leveling";
const TYPES: [&str; 5] = ["manhwa", "manga", "manhua", "webtoon", "comic"];

type Tested = Toonz<ToonzTo>;

// The runner keeps a value that is set to `Null`, so each default is written out.
fn content(rating: &str, types: &[&str], excluded: &[&str]) {
	let list =
		|values: &[&str]| DefaultValue::StringArray(values.iter().map(|v| v.to_string()).collect());
	defaults_set("contentRating", DefaultValue::String(rating.into()));
	defaults_set("types", list(types));
	defaults_set("excludedGenres", list(excluded));
}

fn reset() {
	content("suggestive", &TYPES, &[]);
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

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
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

#[aidoku_test]
fn review_content_rating_defaults_to_suggestive() {
	assert_eq!(ToonzTo.params().rating, "suggestive");
	assert!(include_str!("../res/settings.json").contains(r#""default": "suggestive""#));
}

#[aidoku_test]
fn review_home_rows_keep_the_requested_order() {
	let titles: Vec<_> = ToonzTo
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
			"Trending Manga",
			"Trending Manhwa",
			"Popular Updates Manga",
			"Popular Updates Manhwa",
		]
	);
}

#[aidoku_test]
fn browse_pages_and_sorts() {
	reset();
	let first = search(None, Vec::new());
	assert_eq!(first.entries.len(), 24);
	assert!(first.has_next_page);
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
fn search_text_and_author() {
	reset();
	let result = search(Some("solo leveling"), Vec::new());
	assert!(keys(&result).contains(&SERIES.into()));
	assert!(search(Some("zzzxqvqzzk"), Vec::new()).entries.is_empty());
	let result = search(
		None,
		vec![FilterValue::Text {
			id: "author".into(),
			value: "Chugong".into(),
		}],
	);
	assert!(!result.entries.is_empty());
	assert!(
		result
			.entries
			.iter()
			.all(|manga| manga.title.contains("Solo Leveling"))
	);
}

#[aidoku_test]
fn filters_narrow_the_catalogue() {
	reset();
	let all = keys(&search(None, Vec::new()));
	for filter in [
		select("status", "completed"),
		multi("genre", &["action"], &[]),
		multi("genre", &[], &["romance"]),
		multi("type", &["manhua"], &[]),
		select("rating", "safe"),
	] {
		let narrowed = search(None, vec![filter]);
		assert!(!narrowed.entries.is_empty());
		assert_ne!(keys(&narrowed), all);
	}
}

#[aidoku_test]
fn ratings_follow_the_setting_and_filter() {
	content("safe", &TYPES, &[]);
	let safe = search(None, vec![sort(1)]);
	assert!(
		safe.entries
			.iter()
			.all(|manga| manga.content_rating != ContentRating::NSFW)
	);
	// A level includes every level below it.
	let explicit = search(None, vec![sort(1), select("rating", "pornographic")]);
	assert!(
		explicit
			.entries
			.iter()
			.any(|manga| manga.content_rating == ContentRating::NSFW)
	);
}

#[aidoku_test]
fn settings_types_and_genres_apply_and_yield_to_search() {
	content("erotica", &[], &[]);
	assert!(search(None, Vec::new()).entries.is_empty());
	let override_types = search(None, vec![multi("type", &["manga"], &[])]);
	assert!(!override_types.entries.is_empty());

	content("erotica", &TYPES, &["romance"]);
	let included = search(None, vec![multi("genre", &["romance"], &[])]);
	assert!(
		!included.entries.is_empty(),
		"an included tag must override its exclusion"
	);
	reset();
}

#[aidoku_test]
fn home_loads_every_row() {
	reset();
	let layout = Tested::new().get_home().unwrap();
	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	let expected: Vec<_> = ToonzTo
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
			// Some cards print no update time.
			HomeComponentValue::MangaChapterList { entries, .. } => {
				assert!(
					entries
						.iter()
						.all(|entry| entry.chapter.chapter_number.is_some())
				);
				let dated = entries
					.iter()
					.filter(|entry| entry.chapter.date_uploaded.is_some());
				assert!(dated.count() * 2 > entries.len());
			}
			HomeComponentValue::Scroller { entries, .. } => assert!(!entries.is_empty()),
			_ => panic!("unexpected component"),
		}
	}
}

#[aidoku_test]
fn listing_is_the_latest_sort() {
	reset();
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
	// The latest order shifts between two requests.
	let latest = keys(&search(None, Vec::new()));
	let shared = keys(&listing)
		.iter()
		.filter(|key| latest.contains(key))
		.count();
	assert!(shared >= 20, "only {shared} shared");
}

#[aidoku_test]
fn deep_links() {
	let source = Tested::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert!(matches!(
		link("https://toonz.to/manhwa/solo-leveling"),
		Some(DeepLinkResult::Manga { key }) if key == SERIES
	));
	assert!(matches!(
		link("https://toonz.to/manhwa/solo-leveling/chapter/2"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == SERIES && key == "2"
	));
	assert!(link("https://toonz.to/manhwa/browse?sort=latest").is_none());
}

// The series and chapter pages sit behind a Cloudflare check, which the app solves.
#[aidoku_test]
fn details_chapters_and_pages() {
	let source = Tested::new();
	let manga = source
		.get_manga_update(
			Manga {
				key: SERIES.into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "Solo Leveling");
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() > 100);
	let chapter = chapters.last().unwrap().clone();
	let pages = source
		.get_page_list(
			Manga {
				key: SERIES.into(),
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
	assert!(ids.iter().any(|id| id == "action"));
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
