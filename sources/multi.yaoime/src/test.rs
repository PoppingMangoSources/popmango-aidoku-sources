#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, Home, HomeComponentValue, MangaStatus, Viewer,
	alloc::string::ToString,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use home::{Layout, SECTIONS};
use models::ChapterData;
use serde_json::{Value, json};

const SERIES: &str = "4578";
// One of the series' source ids, which opens the same title.
const SOURCE_SERIES: &str = "196432";
const CHAPTER: &str = "4578/5488466";

// The runner keeps a value that is set to `Null`, so each default is written out.
fn reset() {
	defaults_set("languages", strings(&["en"]));
	defaults_set("excludedTypes", strings(&[]));
	defaults_set("contentRatings", strings(&settings::CONTENT_RATINGS));
	defaults_set("excludedGenres", strings(&[]));
	for group in settings::TAG_GROUPS {
		defaults_set(&format!("excludedTags.{group}"), strings(&[]));
	}
	defaults_set("titlePreference", DefaultValue::String("site".into()));
	defaults_set("officialOnly", DefaultValue::Bool(false));
	defaults_set("cleanChapters", DefaultValue::Bool(false));
	defaults_set("homeSections", strings(&SECTIONS.map(|(id, ..)| id)));
}

fn strings(values: &[&str]) -> DefaultValue {
	DefaultValue::StringArray(values.iter().map(|value| value.to_string()).collect())
}

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
}

fn sort(index: i32, ascending: bool) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending,
	}
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	YaoiMe::new()
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

fn param(url: &str, name: &str) -> Option<String> {
	url.split_once('?')?
		.1
		.split('&')
		.find_map(|pair| pair.strip_prefix(&format!("{name}=")).map(Into::into))
}

fn group(
	label: &str,
	num: Option<f32>,
	vol: Option<f32>,
	extra: bool,
	variants: &[Value],
) -> Value {
	json!({
		"key": label,
		"label": label,
		"num": num,
		"numReal": u8::from(num.is_some()),
		"vol": vol,
		"langs": ["en"],
		"createdAt": 1_794_850_311_000_i64,
		"extra": extra,
		"variants": variants,
	})
}

fn variant(s: u64, id: u64, lang: &str) -> Value {
	json!({ "id": id, "s": s, "nPages": 10, "lang": lang, "b": 1 })
}

fn chapter_fixture() -> ChapterData {
	serde_json::from_value(json!({
		"groups": [
			group("Ch.11 : The End", Some(11.0), None, false, &[variant(7, 111, "en"), variant(8, 112, "en")]),
			group("Hiatus", Some(10.5), None, true, &[variant(7, 105, "en")]),
			group("Ch.10.5", Some(10.5), None, false, &[variant(8, 106, "en")]),
			group("Ch.10", Some(10.0), None, false, &[variant(7, 100, "en"), variant(9, 101, "ja")]),
			group("Vol.2 Ch.1", Some(1.0), Some(2.0), false, &[variant(7, 21, "en")]),
			group("Vol.1 Ch.1", Some(1.0), Some(1.0), false, &[variant(7, 11, "en")]),
			group("Notice", None, None, false, &[variant(7, 1, "en")]),
			group("Ch.2", Some(2.0), None, false, &[variant(9, 2, "ja")]),
		],
		"sources": { "7": "first", "8": "second", "9": "third" },
	}))
	.unwrap()
}

#[aidoku_test]
fn review_home_layout() {
	let order = SECTIONS.map(|(id, name, layout)| {
		let layout = match layout {
			Layout::Featured => "big scroller",
			Layout::Row => "scroller",
			Layout::Chapters => "chapter list",
			Layout::Ranked => "ranked list",
		};
		(id, name, layout)
	});
	assert_eq!(
		order,
		[
			("trending", "Trending now", "big scroller"),
			("popular", "Popular updates", "scroller"),
			("top-rated", "Top rated", "scroller"),
			("rising", "Rising", "scroller"),
			("latest", "Latest releases", "chapter list"),
			("bookmarked", "Most bookmarked", "scroller"),
			("hidden-gems", "Hidden gems", "scroller"),
			("binge", "Binge-worthy", "scroller"),
			("talked", "Most talked about", "ranked list"),
		]
	);
}

#[aidoku_test]
fn review_filter_capabilities() {
	let filters: Vec<Value> = serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	assert_eq!(filters[0]["type"], "sort");
	assert_eq!(
		filters[0]["options"].as_array().unwrap().len(),
		browse::SORTS.len()
	);
	for filter in &filters {
		if filter["type"] == "multi-select" {
			// Genres and tags send both their included and excluded values.
			assert_eq!(filter["canExclude"], true, "{}", filter["id"]);
		}
	}
	let tag_groups: Vec<&str> = filters
		.iter()
		.filter_map(|filter| filter["id"].as_str()?.strip_prefix("tags."))
		.collect();
	assert_eq!(tag_groups, settings::TAG_GROUPS);
	let settings_json: Value = serde_json::from_str(include_str!("../res/settings.json")).unwrap();
	let home = &settings_json.as_array().unwrap().last().unwrap()["items"][0];
	assert_eq!(home["key"], "homeSections");
	let defaults: Vec<&str> = home["default"]
		.as_array()
		.unwrap()
		.iter()
		.map(|value| value.as_str().unwrap())
		.collect();
	assert_eq!(defaults, SECTIONS.map(|(id, ..)| id));
}

#[aidoku_test]
fn review_chapter_list() {
	let languages = ["en".to_string()];
	let names = [("7".to_string(), "Seven".to_string())].into();
	let chapters = chapter_list(chapter_fixture(), &languages, &names, false);
	let keys: Vec<&str> = chapters
		.iter()
		.map(|chapter| chapter.key.as_str())
		.collect();
	assert_eq!(
		keys,
		[
			"7/111", "8/112", "7/105", "8/106", "7/100", "7/21", "7/11", "7/1"
		]
	);
	assert_eq!(chapters[0].title.as_deref(), Some("The End"));
	assert_eq!(chapters[0].chapter_number, Some(11.0));
	assert_eq!(chapters[0].scanlators, Some(vec!["Seven".to_string()]));
	assert_eq!(chapters[1].scanlators, None);
	assert_eq!(chapters[3].title, None);
	assert_eq!(chapters[5].volume_number, Some(2.0));
	assert_eq!(chapters[7].title.as_deref(), Some("Notice"));
	assert_eq!(chapters[7].chapter_number, None);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://yaoi.me/chapter/7/111")
	);
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
}

#[aidoku_test]
fn review_extra_numbers_and_missing_dates() {
	// Shapes from the site's own data: an extra counted as 164, a numbered extra, an undated upload.
	let data: ChapterData = serde_json::from_value(json!({
		"groups": [
			{ "label": "Ch.8", "num": 8, "numReal": 1, "createdAt": 1_794_850_311_000_i64, "variants": [variant(7, 8, "en")] },
			{ "label": "Bounus.7 : Mature official", "num": 164, "numReal": 1, "extra": true, "after": 7, "createdAt": 1_794_850_311_000_i64, "variants": [variant(7, 164, "en")] },
			{ "label": "Ch.26.5", "num": 26.5, "numReal": 1, "extra": true, "after": 26.1, "createdAt": 0, "variants": [variant(7, 265, "en")] },
			{ "label": "hitaus. : Hiatus Notice", "num": 18, "numReal": 0, "extra": true, "createdAt": 0, "variants": [variant(7, 18, "en")] },
		],
	}))
	.unwrap();
	let chapters = chapter_list(data, &["en".to_string()], &BTreeMap::new(), false);
	let rows: Vec<(Option<f32>, Option<&str>, Option<i64>)> = chapters
		.iter()
		.map(|chapter| {
			(
				chapter.chapter_number,
				chapter.title.as_deref(),
				chapter.date_uploaded,
			)
		})
		.collect();
	assert_eq!(
		rows,
		[
			(Some(8.0), None, Some(1_794_850_311)),
			(
				None,
				Some("Bounus.7 : Mature official"),
				Some(1_794_850_311)
			),
			(Some(26.5), None, None),
			(None, Some("hitaus. : Hiatus Notice"), None),
		]
	);
}

#[aidoku_test]
fn review_clean_chapter_list() {
	let languages = ["en".to_string()];
	let names = [("7".to_string(), "Seven".to_string())].into();
	let chapters = chapter_list(chapter_fixture(), &languages, &names, true);
	let rows: Vec<(&str, Option<f32>, Option<f32>)> = chapters
		.iter()
		.map(|chapter| {
			(
				chapter.key.as_str(),
				chapter.chapter_number,
				chapter.volume_number,
			)
		})
		.collect();
	// Extras and unnumbered notices drop out; 10.5 joins 10, which opens its own version.
	assert_eq!(
		rows,
		[
			("7/111", Some(11.0), None),
			("7/100", Some(10.0), None),
			("7/21", Some(1.0), Some(2.0)),
			("7/11", Some(1.0), Some(1.0)),
		]
	);
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.title.is_none() && chapter.scanlators.is_none())
	);

	// A language with only extras keeps the full list.
	let only_extras: ChapterData = serde_json::from_value(json!({
		"groups": [group("Hiatus", Some(3.0), None, true, &[variant(7, 3, "en")])],
	}))
	.unwrap();
	assert_eq!(chapter_list(only_extras, &languages, &names, true).len(), 1);
}

#[aidoku_test]
fn review_chapter_labels() {
	let split = |label| models::split_label(label);
	assert_eq!(
		split("Chapter 1: Oneshot"),
		(Some(1.0), Some("Oneshot".into()))
	);
	assert_eq!(split("Ch.223"), (Some(223.0), None));
	assert_eq!(
		split("Vol.2 Ch.10.5 - Side"),
		(Some(10.5), Some("Side".into()))
	);
	assert_eq!(split("Epilogue"), (None, Some("Epilogue".into())));
	// A title that opens with a number keeps it.
	assert_eq!(split("12 Days Later"), (None, Some("12 Days Later".into())));
	assert_eq!(
		split("Ch.12 Days Later"),
		(Some(12.0), Some("Days Later".into()))
	);
	assert_eq!(split("Chapter12"), (Some(12.0), None));
}

#[aidoku_test]
fn review_reader_cipher() {
	assert_eq!(
		cipher::sign("/api/reader/chapter?s=4578&c=5488466", "1794900000000"),
		"DSbXD1Cf0Hk4cSdna5uxHo40kD8EKxJOBCGD5pNeYII"
	);
	assert_eq!(
		cipher::unseal(
			"AAECAwQFBgcICQoLDA0ODxAREhMUFRYXGBkaG0EJ4ND8briDlYRAhUK3FE2ykAEwNZ67SzTLjdtwkuu8j5-EYQc"
		)
		.unwrap(),
		["aa11", "bb22"]
	);
	assert!(cipher::unseal("AAEC").is_err());
}

#[aidoku_test]
fn review_links() {
	assert!(matches!(
		helpers::parse_link("https://yaoi.me/series/4578/melting-point-bl"),
		Some(Link::Series(key)) if key == "4578"
	));
	assert!(matches!(
		helpers::parse_link("https://yaoi.me/series/4578?tab=chapters"),
		Some(Link::Series(key)) if key == "4578"
	));
	assert!(matches!(
		helpers::parse_link("https://yaoi.me/chapter/196432/5488437?on=kagane"),
		Some(Link::Chapter(key)) if key == "196432/5488437"
	));
	assert!(helpers::parse_link("https://yaoi.me/author/Pibi").is_none());
	assert!(helpers::parse_link("https://yaoi.me/series/melting").is_none());
	assert!(helpers::parse_link("https://yaoi.me.example.com/series/4578").is_none());
}

#[aidoku_test]
fn review_search_overrides_settings() {
	reset();
	defaults_set("excludedTypes", strings(&["manhwa"]));
	defaults_set("contentRatings", strings(&["safe"]));
	defaults_set("excludedGenres", strings(&["Romance", "Drama"]));
	let mut search = BrowseParams::new(Some("kiss".into()));
	assert_eq!(search.settings.hidden_types, ["manhwa"]);
	assert_eq!(
		search.settings.hidden_ratings,
		["suggestive", "erotica", "pornographic"]
	);
	search.filter(select("type", "manhwa"));
	search.filter(select("content", "erotica"));
	search.filter(FilterValue::MultiSelect {
		id: "genre".into(),
		included: vec!["Romance".into()],
		excluded: vec!["Comedy".into()],
	});
	search.filter(sort(5, true));
	search.filter(FilterValue::Range {
		id: "year".into(),
		from: Some(2020.0),
		to: None,
	});
	assert!(search.settings.hidden_types.is_empty());
	assert!(search.settings.hidden_ratings.is_empty());
	assert_eq!(search.settings.hidden_tags, ["drama"]);
	let url = search.url(2);
	assert_eq!(param(&url, "q").as_deref(), Some("kiss"));
	assert_eq!(param(&url, "langs").as_deref(), Some("en"));
	assert_eq!(param(&url, "sort").as_deref(), Some("az"));
	assert_eq!(param(&url, "dir").as_deref(), Some("asc"));
	assert_eq!(param(&url, "type").as_deref(), Some("manhwa"));
	assert_eq!(param(&url, "genre").as_deref(), Some("Romance"));
	assert_eq!(param(&url, "without").as_deref(), Some("Comedy"));
	assert_eq!(param(&url, "yearFrom").as_deref(), Some("2020"));
	assert_eq!(param(&url, "yearTo"), None);
	assert_eq!(param(&url, "page").as_deref(), Some("2"));
	let cookie = search.settings.cookie().unwrap();
	assert!(cookie.starts_with("yf.adult=1; yf.settings=%7B%22hiddenTypes%22"));
	assert!(cookie.contains("%22titleLang%22%3A%22site%22"));
}

#[aidoku_test]
fn review_cleared_settings_show_nothing() {
	reset();
	defaults_set("contentRatings", strings(&[]));
	assert!(search(None, vec![]).entries.is_empty());
	reset();
	defaults_set("languages", strings(&[]));
	assert!(search(None, vec![]).entries.is_empty());
	assert!(YaoiMe::new().get_home().unwrap().components.is_empty());
	reset();
}

#[aidoku_test]
fn live_browse_paging() {
	reset();
	let source = YaoiMe::new();
	let first = source.get_search_manga_list(None, 1, vec![]).unwrap();
	let second = source.get_search_manga_list(None, 2, vec![]).unwrap();
	assert_eq!(first.entries.len(), 60);
	assert!(first.has_next_page && second.has_next_page);
	assert_ne!(keys(&first), keys(&second));
	assert!(first.entries.iter().all(|manga| {
		!manga.title.is_empty()
			&& manga
				.cover
				.as_deref()
				.is_some_and(|cover| cover.starts_with("https://yaoi.me/i/c/"))
			&& manga.description.is_none()
			&& manga.tags.is_none()
	}));
}

#[aidoku_test]
fn live_sorts() {
	reset();
	for index in 0..browse::SORTS.len() as i32 {
		let descending = search(None, vec![sort(index, false)]);
		assert!(!descending.entries.is_empty(), "sort {index}");
		if index > 0 {
			let ascending = search(None, vec![sort(index, true)]);
			assert_ne!(keys(&descending), keys(&ascending), "sort {index}");
		}
	}
}

#[aidoku_test]
fn live_filters() {
	reset();
	// Most of the newest updates fall within any period, so the filters are compared by popularity.
	let all = keys(&search(None, vec![sort(2, false)]));
	let novels = search(None, vec![select("type", "novel")]);
	assert!(!novels.entries.is_empty() && novels.entries.len() < 60);
	let nothing = search(
		None,
		vec![FilterValue::MultiSelect {
			id: "genre".into(),
			included: vec!["Mecha".into(), "Yuri".into()],
			excluded: vec![],
		}],
	);
	assert!(nothing.entries.is_empty());
	for filter in [
		select("status", "hiatus"),
		select("demographic", "Seinen"),
		select("updated", "1"),
		FilterValue::Check {
			id: "anime".into(),
			value: 1,
		},
		FilterValue::Check {
			id: "licensed".into(),
			value: 1,
		},
		FilterValue::MultiSelect {
			id: "tags.sexual".into(),
			included: vec!["Anal Sex".into()],
			excluded: vec![],
		},
		FilterValue::MultiSelect {
			id: "genre".into(),
			included: vec![],
			excluded: vec!["Doujinshi".into()],
		},
		FilterValue::Range {
			id: "rating".into(),
			from: Some(90.0),
			to: None,
		},
		FilterValue::Text {
			id: "pages".into(),
			value: "500".into(),
		},
	] {
		let label = format!("{filter:?}");
		let narrowed = keys(&search(None, vec![sort(2, false), filter]));
		assert!(!narrowed.is_empty(), "{label}");
		assert_ne!(narrowed, all, "{label}");
	}
	assert!(search(Some("zzqqxxnomatch"), vec![]).entries.is_empty());
}

#[aidoku_test]
fn live_content_ratings() {
	reset();
	for rating in settings::CONTENT_RATINGS {
		defaults_set("contentRatings", strings(&[rating]));
		assert!(!search(None, vec![]).entries.is_empty(), "{rating}");
	}
	// The Content rating filter overrides the setting for one search.
	defaults_set("contentRatings", strings(&["safe"]));
	let safe = keys(&search(None, vec![]));
	let pornographic = keys(&search(None, vec![select("content", "pornographic")]));
	assert!(!pornographic.is_empty());
	assert_ne!(safe, pornographic);
	reset();
}

#[aidoku_test]
fn live_excluded_type_overridden_by_filter() {
	reset();
	defaults_set("excludedTypes", strings(&["novel"]));
	assert!(
		!search(None, vec![select("type", "novel")])
			.entries
			.is_empty()
	);
	defaults_set("excludedGenres", strings(&["Romance"]));
	let tapped = search(None, vec![select("genre", "Romance")]);
	assert!(!tapped.entries.is_empty());
	reset();
}

#[aidoku_test]
fn live_title_language() {
	reset();
	let listed = search(None, vec![sort(2, false)]);
	defaults_set("titlePreference", DefaultValue::String("native".into()));
	let native = search(None, vec![sort(2, false)]);
	assert_eq!(keys(&listed), keys(&native));
	assert!(
		listed
			.entries
			.iter()
			.zip(&native.entries)
			.any(|(listed, native)| listed.title != native.title)
	);
	reset();
}

#[aidoku_test]
fn live_author_search() {
	reset();
	let found = search(
		None,
		vec![FilterValue::Text {
			id: "author".into(),
			value: "Pibi".into(),
		}],
	);
	assert!(found.entries.iter().any(|manga| manga.key == SERIES));
}

#[aidoku_test]
fn live_pasted_link() {
	reset();
	let found = search(
		Some("https://yaoi.me/series/196432/melting-point-yaoi"),
		vec![],
	);
	assert_eq!(keys(&found), [SERIES]);
	assert!(found.entries[0].description.is_none() && found.entries[0].authors.is_none());
	// An unknown series falls back to ordinary search of the pasted text.
	let fallback = search(Some("https://yaoi.me/series/999999999"), vec![]);
	assert!(
		fallback
			.entries
			.iter()
			.all(|manga| manga.key != "999999999")
	);
}

#[aidoku_test]
fn live_details() {
	reset();
	let manga = YaoiMe::new()
		.get_manga_update(
			Manga {
				key: SERIES.into(),
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert_eq!(manga.title, "Melting Point");
	assert_eq!(manga.authors, Some(vec!["Pibi".to_string()]));
	assert_eq!(manga.artists, Some(vec!["Pibi".to_string()]));
	assert!(manga.description.unwrap().starts_with("After twelve years"));
	assert!(manga.tags.unwrap().iter().any(|tag| tag == "Drama"));
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert!(manga.cover.unwrap().starts_with("https://yaoi.me/i/c/"));
	assert!(manga.chapters.is_none());

	let split = YaoiMe::new()
		.get_manga_update(
			Manga {
				key: "5239".into(),
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert_eq!(
		split.authors,
		Some(vec!["Gilmong".to_string(), "Ranmaru".to_string()])
	);
	assert_eq!(split.artists, Some(vec!["Bugi".to_string()]));
}

#[aidoku_test]
fn live_chapters() {
	reset();
	let source = YaoiMe::new();
	let key_only = |key: &str| Manga {
		key: key.into(),
		..Default::default()
	};
	let all = source
		.get_manga_update(key_only(SERIES), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(all.len() > 100);
	assert!(
		all.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
	assert!(
		all.iter()
			.any(|chapter| chapter.scanlators.as_deref() == Some(&["Official".to_string()]))
	);
	assert!(all.iter().any(|chapter| chapter.key == CHAPTER));

	// 4-Week Lovers ends at 133; its "Bounus" extras are counted 158 to 164 by the site.
	let extras = source
		.get_manga_update(key_only("4971"), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(
		extras
			.iter()
			.all(|chapter| chapter.chapter_number.is_none_or(|number| number <= 133.0))
	);
	assert!(extras.iter().any(|chapter| {
		chapter
			.title
			.as_deref()
			.is_some_and(|title| title.starts_with("Bounus.7"))
	}));

	defaults_set("officialOnly", DefaultValue::Bool(true));
	let official = source
		.get_manga_update(key_only(SERIES), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(!official.is_empty() && official.len() < all.len());

	defaults_set("cleanChapters", DefaultValue::Bool(true));
	let clean = source
		.get_manga_update(key_only(SERIES), false, true)
		.unwrap()
		.chapters
		.unwrap();
	let mut numbers: Vec<i32> = clean
		.iter()
		.map(|chapter| chapter.chapter_number.unwrap() as i32)
		.collect();
	let count = numbers.len();
	numbers.dedup();
	assert_eq!(numbers.len(), count);
	assert!(
		clean
			.iter()
			.all(|chapter| chapter.title.is_none() && chapter.scanlators.is_none())
	);
	reset();
}

#[aidoku_test]
fn live_pages() {
	reset();
	let pages = YaoiMe::new()
		.get_page_list(
			Manga {
				key: SERIES.into(),
				..Default::default()
			},
			Chapter {
				key: CHAPTER.into(),
				..Default::default()
			},
		)
		.unwrap();
	assert!(pages.len() > 10);
	assert!(pages.iter().all(|page| matches!(
		&page.content,
		PageContent::Url(url, _) if url.starts_with("https://yaoi.me/i/p/")
	)));
	assert!(
		YaoiMe::new()
			.get_page_list(
				Manga::default(),
				Chapter {
					key: "999999/1".into(),
					..Default::default()
				}
			)
			.is_err()
	);
}

#[aidoku_test]
fn live_deep_links() {
	reset();
	let source = YaoiMe::new();
	let series = source
		.handle_deep_link(format!(
			"https://yaoi.me/series/{SOURCE_SERIES}/melting-point-yaoi"
		))
		.unwrap();
	assert!(matches!(series, Some(DeepLinkResult::Manga { key }) if key == SERIES));
	let chapter = source
		.handle_deep_link("https://yaoi.me/chapter/196432/5488437".into())
		.unwrap();
	assert!(matches!(
		chapter,
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == SERIES && key == "196432/5488437"
	));
	assert!(
		source
			.handle_deep_link("https://yaoi.me/forum/thread/5".into())
			.unwrap()
			.is_none()
	);
}

#[aidoku_test]
fn live_unknown_series() {
	reset();
	assert!(
		YaoiMe::new()
			.get_manga_update(
				Manga {
					key: "999999999".into(),
					..Default::default()
				},
				true,
				true,
			)
			.is_err()
	);
}

#[aidoku_test]
fn live_home() {
	reset();
	let home = YaoiMe::new().get_home().unwrap();
	let titles: Vec<&str> = home
		.components
		.iter()
		.map(|component| component.title.as_deref().unwrap())
		.collect();
	assert_eq!(titles, SECTIONS.map(|(_, name, _)| name));
	for component in &home.components {
		match &component.value {
			HomeComponentValue::BigScroller { entries, .. } => {
				assert!(!entries.is_empty());
				assert!(
					entries
						.iter()
						.all(|manga| manga.tags.as_ref().is_some_and(|tags| tags.len() == 1))
				);
			}
			HomeComponentValue::Scroller { entries, .. } => {
				assert!(!entries.is_empty(), "{:?}", component.title);
				assert!(entries.iter().all(|link| link.subtitle.is_none()));
			}
			HomeComponentValue::MangaChapterList {
				page_size, entries, ..
			} => {
				assert_eq!(*page_size, Some(5));
				assert!(
					entries
						.iter()
						.all(|entry| (entry.chapter.chapter_number.is_some()
							|| entry.chapter.title.is_some())
							&& entry.chapter.key.contains('/'))
				);
				assert!(!entries.is_empty());
			}
			HomeComponentValue::MangaList {
				ranking,
				page_size,
				entries,
				..
			} => {
				assert!(*ranking);
				assert_eq!(*page_size, Some(4));
				assert!(!entries.is_empty());
			}
			_ => panic!("unexpected component"),
		}
	}

	defaults_set("homeSections", strings(&["talked", "trending"]));
	let home = YaoiMe::new().get_home().unwrap();
	let titles: Vec<&str> = home
		.components
		.iter()
		.map(|component| component.title.as_deref().unwrap())
		.collect();
	assert_eq!(titles, ["Trending now", "Most talked about"]);
	reset();
}

#[aidoku_test]
fn live_listings() {
	reset();
	let source = YaoiMe::new();
	for (id, name, _) in SECTIONS.iter().filter(|(id, ..)| *id != "popular") {
		let result = source
			.get_manga_list(
				Listing {
					id: (*id).into(),
					name: (*name).into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		assert!(!result.entries.is_empty(), "{id}");
	}
}
