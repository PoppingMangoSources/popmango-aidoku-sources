#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Listing,
	ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;

const SERIES: &str = "rockstar-never-die-vwe1q";
const TYPES: [&str; 5] = ["manhwa", "manga", "manhua", "webtoon", "comic"];

type Tested = Toonz<LewdManhwa>;

// The runner keeps a value that is set to `Null`, so each default is written out.
fn content(rating: &str, types: &[&str], excluded: &[&str]) {
	let list =
		|values: &[&str]| DefaultValue::StringArray(values.iter().map(|v| v.to_string()).collect());
	defaults_set("contentRating", DefaultValue::String(rating.into()));
	defaults_set("types", list(types));
	defaults_set("excludedGenres", list(excluded));
}

fn reset() {
	content("pornographic", &TYPES, &[]);
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

fn manga(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

#[aidoku_test]
fn review_home_rows_keep_the_site_order() {
	let titles: Vec<_> = LewdManhwa
		.params()
		.home_rows
		.iter()
		.map(|(title, _)| *title)
		.collect();
	assert_eq!(titles, ["Featured", "Latest Updates"]);
}

#[aidoku_test]
fn browse_pages_and_sorts() {
	reset();
	let first = search(None, Vec::new());
	assert!(first.entries.len() >= 20);
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
	let result = search(Some("rockstar never die"), Vec::new());
	assert!(keys(&result).contains(&SERIES.into()));
	assert!(search(Some("zzzxqvqzzk"), Vec::new()).entries.is_empty());
	let result = search(
		None,
		vec![FilterValue::Text {
			id: "author".into(),
			value: "Jiong".into(),
		}],
	);
	assert!(keys(&result).contains(&SERIES.into()));
}

#[aidoku_test]
fn filters_narrow_the_catalogue() {
	reset();
	let all = keys(&search(None, Vec::new()));
	for filter in [
		select("status", "completed"),
		multi("genre", &["yaoi"], &[]),
		multi("genre", &[], &["yaoi"]),
		multi("type", &["manga"], &[]),
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
	let safe = keys(&search(None, Vec::new()));
	reset();
	let explicit = keys(&search(None, Vec::new()));
	assert_ne!(safe, explicit);
	content("safe", &TYPES, &[]);
	// A level includes every level below it.
	let widened = search(None, vec![select("rating", "pornographic")]);
	assert_eq!(keys(&widened), explicit);
	reset();
}

#[aidoku_test]
fn settings_types_and_genres_apply_and_yield_to_search() {
	content("pornographic", &[], &[]);
	assert!(search(None, Vec::new()).entries.is_empty());
	assert!(
		!search(None, vec![multi("type", &["manhwa"], &[])])
			.entries
			.is_empty()
	);

	// The site answers nothing for a genre its cookie blocks, so results prove the override.
	reset();
	let yaoi = keys(&search(None, vec![multi("genre", &["yaoi"], &[])]));
	assert!(!yaoi.is_empty());
	assert_eq!(
		keys(&search(None, vec![select("genre", "Yaoi")])),
		yaoi,
		"a tapped tag searches by its slug"
	);
	content("pornographic", &TYPES, &["yaoi"]);
	let included = search(None, vec![multi("genre", &["yaoi"], &[])]);
	assert_eq!(
		keys(&included),
		yaoi,
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
	assert_eq!(titles, ["Featured", "Latest Updates"]);
	for component in &layout.components {
		match &component.value {
			HomeComponentValue::BigScroller { entries, .. } => assert!(
				entries.len() > 1
					&& entries
						.iter()
						.all(|manga| manga.description.is_some() && manga.cover.is_some())
			),
			HomeComponentValue::MangaChapterList { entries, .. } => assert!(
				!entries.is_empty()
					&& entries
						.iter()
						.all(|entry| entry.chapter.chapter_number.is_some())
			),
			HomeComponentValue::Scroller { entries, .. } => assert!(entries.len() >= 5),
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
	assert!(matches!(
		link("https://lewdmanhwa.com/webtoon/rockstar-never-die-vwe1q"),
		Some(DeepLinkResult::Manga { key }) if key == SERIES
	));
	assert!(matches!(
		link("https://lewdmanhwa.com/webtoon/rockstar-never-die-vwe1q/chapter/45-wad21"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == SERIES && key == "45-wad21"
	));
	assert!(link("https://lewdmanhwa.com/webtoons?sort=latest").is_none());

	reset();
	let pasted = search(
		Some("https://lewdmanhwa.com/webtoon/rockstar-never-die-vwe1q"),
		Vec::new(),
	);
	assert_eq!(keys(&pasted), [SERIES]);
	assert_eq!(pasted.entries[0].title, "Rockstar Never Die");
	assert!(pasted.entries[0].description.is_none());
	assert!(pasted.entries[0].tags.is_none());
}

#[aidoku_test]
fn details_chapters_and_pages() {
	let source = Tested::new();
	let details = source.get_manga_update(manga(SERIES), true, true).unwrap();
	assert_eq!(details.title, "Rockstar Never Die");
	assert_eq!(details.authors, Some(vec!["Jiong".into()]));
	assert_eq!(details.status, MangaStatus::Ongoing);
	assert_eq!(details.viewer, Viewer::Webtoon);
	assert!(details.tags.unwrap().contains(&"Yaoi".into()));
	assert!(details.description.is_some());
	let chapters = details.chapters.unwrap();
	assert!(chapters.len() >= 47);
	assert!(chapters.iter().any(|chapter| chapter.key == "45-wad21"));

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
				key: "45-wad21".into(),
				..Default::default()
			},
		)
		.unwrap();
	assert!(pages.len() > 10);

	// A home row keyed by the number alone still opens the suffixed chapter.
	let by_number = source
		.get_page_list(
			manga(SERIES),
			Chapter {
				key: "45".into(),
				..Default::default()
			},
		)
		.unwrap();
	assert_eq!(by_number.len(), pages.len());
}

#[aidoku_test]
fn unknown_series_fails() {
	let source = Tested::new();
	assert!(
		source
			.get_manga_update(manga("zzzxqv-no-such-series"), true, true)
			.is_err()
	);
	assert!(
		source
			.get_page_list(
				manga(SERIES),
				Chapter {
					key: "999999".into(),
					..Default::default()
				}
			)
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
	assert!(ids.iter().any(|id| id == "yaoi"));
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
