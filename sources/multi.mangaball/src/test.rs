#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, Home, HomeComponentValue, MangaStatus, Viewer,
	alloc::string::ToString,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use home::{Layout, SECTIONS, fill, section_url};

const SERIES: &str = "6851547b702284f834178357";
const ACTION: &str = "685146c5f3ed681c80f257e3";
const LONG_STRIP: &str = "685148d915e8b86aae68e517";

// The runner keeps a value that is set to `Null`, so each default is written out.
fn reset() {
	defaults_set("languages", strings(&["en"]));
	defaults_set("showAdult", DefaultValue::Bool(false));
	defaults_set("adultOnly", DefaultValue::Bool(false));
	defaults_set("type", DefaultValue::String("any".into()));
	for key in [
		"excludedGenres",
		"excludedThemes",
		"excludedFormats",
		"excludedContent",
		"excludedScanlators",
	] {
		defaults_set(key, strings(&[]));
	}
}

fn strings(values: &[&str]) -> DefaultValue {
	DefaultValue::StringArray(values.iter().map(|value| value.to_string()).collect())
}

fn title(id: &str, language: &str, adult: bool) -> Title {
	serde_json::from_value(serde_json::json!({
		"id": id,
		"name": "Example Title",
		"slug": "example-title",
		"image": {
			"cdn_mangadex": "https://example.com/mirror.jpg",
			"file": null,
			"cover": { "path": "abc\\cover_1.webp" }
		},
		"description": ["First *paragraph*.", "Second."],
		"originalLanguage": language,
		"status": "Ongoing",
		"availableTranslatedLanguages": ["en", "id"],
		"tags": [
			{ "id": LONG_STRIP, "name": "Long Strip" },
			{ "id": ACTION, "name": "Action" }
		],
		"author": [{ "name": "Writer" }, { "name": "Studio" }],
		"is18plus": adult,
		"recent_chapters": [
			{
				"id": "6af713191f3a6ace87d43848",
				"name": "",
				"number": 40,
				"volume": 0,
				"lang": "vi",
				"group": { "name": "Group A" },
				"created_at": "2026-11-13T13:39:37.229000"
			},
			{
				"id": "6af075a8c2f1fe0ea088ea68",
				"name": "Vol.5 Ch.39.5",
				"number": 39.5,
				"volume": 5,
				"lang": "en",
				"group": { "name": "Group B" },
				"created_at": "2026-11-08T13:14:16.408000"
			}
		]
	}))
	.unwrap()
}

fn param(url: &str, name: &str) -> Option<String> {
	url.split_once('?')?
		.1
		.split('&')
		.find_map(|pair| pair.strip_prefix(&format!("{name}=")).map(Into::into))
}

#[aidoku_test]
fn review_home_layout() {
	reset();
	let order = SECTIONS.map(|(_, name, layout)| {
		let layout = match layout {
			Layout::Featured => "big scroller",
			Layout::Ranked => "ranked list",
			Layout::Row => "scroller",
			Layout::Chapters => "chapter list",
		};
		(name, layout)
	});
	assert_eq!(
		order,
		[
			("Featured Manga", "big scroller"),
			("Top Weekly Ranking", "ranked list"),
			("Titles Recommended", "scroller"),
			("Latest Chapter Updates", "chapter list"),
			("Recently Added Titles", "scroller"),
		]
	);

	let search = SearchParams::new(None);
	let week = section_url("top-views", &search);
	assert_eq!(param(&week, "period").as_deref(), Some("week"));
	assert_eq!(param(&week, "adult_mode").as_deref(), Some("no_18"));
	let latest = section_url("lastupdate", &search);
	assert_eq!(param(&latest, "sort_by").as_deref(), Some("lastupdate"));
	assert_eq!(
		param(&section_url("created_at", &search), "sort_by").as_deref(),
		Some("created_at")
	);

	for (id, name, layout) in SECTIONS {
		let titles = vec![title(SERIES, "ko", false)];
		match fill(id, name, layout, titles, &search) {
			HomeComponentValue::BigScroller { entries, .. } => {
				assert!(matches!(layout, Layout::Featured));
				let manga = &entries[0];
				assert_eq!(
					manga.description.as_deref(),
					Some("First *paragraph*.\n\nSecond.")
				);
				assert_eq!(
					manga.authors.as_deref(),
					Some(&["Writer".to_string(), "Studio".to_string()][..])
				);
				assert_eq!(
					manga.tags.as_deref(),
					Some(&["Action".to_string(), "Long Strip".to_string()][..])
				);
				assert_eq!(manga.status, MangaStatus::Ongoing);
				assert_eq!(manga.viewer, Viewer::Webtoon);
			}
			HomeComponentValue::MangaList {
				ranking,
				page_size,
				entries,
				listing,
			} => {
				assert!(matches!(layout, Layout::Ranked));
				assert!(ranking);
				assert_eq!(page_size, Some(3));
				assert!(entries[0].subtitle.is_none());
				assert!(listing.is_none());
			}
			HomeComponentValue::Scroller { entries, listing } => {
				assert!(matches!(layout, Layout::Row));
				assert!(entries[0].subtitle.is_none());
				assert_eq!(listing.map(|listing| listing.id), {
					(id == "created_at").then(|| id.to_string())
				});
			}
			HomeComponentValue::MangaChapterList {
				page_size,
				entries,
				listing,
			} => {
				assert!(matches!(layout, Layout::Chapters));
				assert_eq!(page_size, Some(5));
				let entry = &entries[0];
				assert_eq!(entry.chapter.key, "6af075a8c2f1fe0ea088ea68");
				assert_eq!(entry.chapter.chapter_number, Some(39.5));
				assert_eq!(entry.chapter.volume_number, Some(5.0));
				assert_eq!(entry.chapter.date_uploaded, Some(1_794_143_656));
				assert!(entry.manga.description.is_none() && entry.manga.tags.is_none());
				assert_eq!(listing.unwrap().id, "lastupdate");
			}
			_ => panic!("unexpected component for {name}"),
		}
	}
}

#[aidoku_test]
fn review_cards_are_compact() {
	let manga = Manga::try_from(title(SERIES, "ja", true)).unwrap();
	assert_eq!(manga.key, SERIES);
	assert_eq!(manga.title, "Example Title");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://bulbasaur.poke-black-and-white.net/covers/abc/cover_1.webp")
	);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert!(manga.description.is_none() && manga.authors.is_none() && manga.tags.is_none());

	let details = title(SERIES, "ja", false).into_details().unwrap();
	assert_eq!(details.content_rating, ContentRating::Safe);
	assert_eq!(
		details.url.as_deref(),
		Some("https://mangaball.com/title-detail/6851547b702284f834178357")
	);
}

#[aidoku_test]
fn review_search_language_shapes() {
	for languages in [serde_json::json!("en"), serde_json::json!(["en", "fr"])] {
		let response: models::Paginated<Title> = serde_json::from_value(serde_json::json!({
			"data": [{"id": SERIES, "name": "Example", "availableTranslatedLanguages": languages}],
			"pagination": {"page": 1, "total_pages": 2}
		}))
		.unwrap();
		let parsed = response.data.into_iter().next().unwrap();
		assert_eq!(parsed.translated_languages[0], "en");
		assert_eq!(Manga::try_from(parsed).unwrap().title, "Example");
	}
	assert!(
		serde_json::from_value::<Title>(serde_json::json!({
			"id": SERIES, "name": "Example", "availableTranslatedLanguages": 42
		}))
		.is_err()
	);
}

#[aidoku_test]
fn review_missing_titles_do_not_break_valid_cards() {
	reset();
	let response: models::Paginated<Title> = serde_json::from_value(serde_json::json!({
		"data": [
			{"id": SERIES},
			{"id": SERIES, "name": null},
			{"id": SERIES, "name": "  "},
			{"id": " ", "name": "Example"},
			{"id": SERIES, "name": "Example"}
		],
		"pagination": {"page": 1, "total_pages": 2}
	}))
	.unwrap();
	let entries: Vec<Manga> = response
		.data
		.into_iter()
		.filter_map(|title| title.try_into().ok())
		.collect();
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].key, SERIES);
	for layout in [
		Layout::Featured,
		Layout::Ranked,
		Layout::Row,
		Layout::Chapters,
	] {
		let mut valid = title(SERIES, "ja", false);
		valid.recent_chapters.remove(0);
		let missing: Title = serde_json::from_value(serde_json::json!({
			"id": SERIES, "recent_chapters": [{"id": SERIES, "lang": "en"}]
		}))
		.unwrap();
		let component = fill(
			"lastupdate",
			"Latest Chapter Updates",
			layout,
			vec![missing, valid],
			&SearchParams::new(None),
		);
		let count = match component {
			HomeComponentValue::BigScroller { entries, .. } => entries.len(),
			HomeComponentValue::Scroller { entries, .. }
			| HomeComponentValue::MangaList { entries, .. } => entries.len(),
			HomeComponentValue::MangaChapterList { entries, .. } => entries.len(),
			_ => panic!("unexpected layout"),
		};
		assert_eq!(count, 1);
	}
	let missing: Title = serde_json::from_value(serde_json::json!({"id": SERIES})).unwrap();
	assert!(missing.into_details().is_err());
}

#[aidoku_test]
fn review_search_parameters() {
	reset();
	let browse = SearchParams::new(None).url(2);
	assert_eq!(param(&browse, "sort_by").as_deref(), Some("lastupdate"));
	assert_eq!(param(&browse, "sort_order").as_deref(), Some("desc"));
	assert_eq!(param(&browse, "adult_mode").as_deref(), Some("no_18"));
	assert_eq!(param(&browse, "chapterLanguage").as_deref(), Some("en"));
	assert_eq!(param(&browse, "page").as_deref(), Some("2"));
	assert_eq!(param(&browse, "type"), None);
	assert_eq!(param(&browse, "excluded_tags"), None);

	let keyword = SearchParams::new(Some(" hound ".into())).url(1);
	assert_eq!(param(&keyword, "keyword").as_deref(), Some("hound"));
	assert_eq!(param(&keyword, "sort_by").as_deref(), Some("lastupdate"));

	defaults_set("languages", strings(&["zh", "pt-BR"]));
	defaults_set("showAdult", DefaultValue::Bool(true));
	defaults_set("type", DefaultValue::String("manhwa".into()));
	defaults_set("excludedGenres", strings(&[ACTION]));
	defaults_set("excludedFormats", strings(&[LONG_STRIP]));
	let url = SearchParams::new(None).url(1);
	assert_eq!(
		param(&url, "chapterLanguage").as_deref(),
		Some("cn%2Czh-hk%2Czh-hans%2Czh-hant%2Czh%2Cpt-br")
	);
	assert_eq!(param(&url, "adult_mode").as_deref(), Some("all"));
	assert_eq!(param(&url, "type").as_deref(), Some("manhwa"));
	assert_eq!(
		param(&url, "excluded_tags"),
		Some(format!("{ACTION}%2C{LONG_STRIP}"))
	);

	defaults_set("adultOnly", DefaultValue::Bool(true));
	assert_eq!(settings::adult_mode(), "only_18");
	defaults_set("showAdult", DefaultValue::Bool(false));
	assert_eq!(settings::adult_mode(), "no_18");
	reset();
}

#[aidoku_test]
fn review_feed_settings() {
	reset();
	let search = SearchParams::new(None);
	assert!(search.allows(&title(SERIES, "ko", false)));
	assert!(!search.allows(&title(SERIES, "ko", true)));

	defaults_set("showAdult", DefaultValue::Bool(true));
	assert!(SearchParams::new(None).allows(&title(SERIES, "ko", true)));
	defaults_set("adultOnly", DefaultValue::Bool(true));
	assert!(!SearchParams::new(None).allows(&title(SERIES, "ko", false)));
	reset();

	defaults_set("type", DefaultValue::String("manga".into()));
	assert!(!SearchParams::new(None).allows(&title(SERIES, "ko", false)));
	assert!(SearchParams::new(None).allows(&title(SERIES, "jp", false)));
	reset();

	defaults_set("excludedGenres", strings(&[ACTION]));
	assert!(!SearchParams::new(None).allows(&title(SERIES, "ko", false)));
	reset();

	defaults_set("languages", strings(&["fr"]));
	assert!(!SearchParams::new(None).allows(&title(SERIES, "ko", false)));
	reset();
}

#[aidoku_test]
fn review_chapters() {
	let languages = site_languages(vec!["en".into(), "ja".into()]);
	let chapter = |value: serde_json::Value| {
		serde_json::from_value::<ChapterItem>(value)
			.unwrap()
			.into_chapter(&languages, &[])
	};
	let labelled = chapter(serde_json::json!({
		"id": "a", "name": "Chapter 188", "number": 188, "volume": 0, "lang": "en",
		"group": { "name": "Group A" }, "created_at": "2026-11-08T17:07:41.732000"
	}))
	.unwrap();
	assert_eq!(labelled.title, None);
	assert_eq!(labelled.chapter_number, Some(188.0));
	assert_eq!(labelled.volume_number, None);
	assert_eq!(labelled.language.as_deref(), Some("en"));
	assert_eq!(
		labelled.scanlators.as_deref(),
		Some(&["Group A".to_string()][..])
	);
	assert_eq!(
		labelled.url.as_deref(),
		Some("https://mangaball.com/chapter-detail/a")
	);

	let named =
		chapter(serde_json::json!({ "id": "b", "name": "Epilogue", "number": 12, "lang": "JP" }))
			.unwrap();
	assert_eq!(named.title.as_deref(), Some("Epilogue"));
	assert_eq!(named.language.as_deref(), Some("ja"));

	assert!(
		chapter(serde_json::json!({ "id": "c", "name": "1", "number": 1, "lang": "id" })).is_none()
	);
	assert!(chapter(serde_json::json!({ "id": " ", "lang": "en" })).is_none());
}

#[aidoku_test]
fn review_links() {
	let key = |url: &str| match parse_link(url) {
		Some(Link::Title(id)) => format!("title {id}"),
		Some(Link::Chapter(id)) => format!("chapter {id}"),
		None => "none".into(),
	};
	assert_eq!(
		key("https://mangaball.com/title-detail/6851547b702284f834178357-example-title/?a=1"),
		format!("title {SERIES}")
	);
	assert_eq!(
		key("https://mangaball.com/title-detail/6851547b702284f834178357"),
		format!("title {SERIES}")
	);
	assert_eq!(
		key("https://mangaball.com/chapter-detail/6af082f2a91bdf88681d2971#page"),
		"chapter 6af082f2a91bdf88681d2971"
	);
	assert_eq!(
		key("https://mangaball.com/title-detail/example-title"),
		"none"
	);
	assert_eq!(key("https://mangaball.com/search-advanced"), "none");
	assert_eq!(key("hound"), "none");
	assert_eq!(
		key("https://example.com/title-detail/6851547b702284f834178357"),
		"none"
	);
	assert_eq!(
		key("https://mangaball.com.example.com/title-detail/6851547b702284f834178357"),
		"none"
	);
}

#[aidoku_test]
fn search_pages_and_filters() {
	reset();
	let source = MangaBall::new();
	let first = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(first.has_next_page);
	assert!(!first.entries.is_empty() && first.entries.len() <= 24);
	assert!(first.entries.iter().all(|manga| {
		!manga.key.is_empty() && !manga.title.is_empty() && manga.description.is_none()
	}));
	assert_ne!(first.entries[0].key, second.entries[0].key);

	let nothing = source
		.get_search_manga_list(Some("zzqxjvnosuchtitlezzqxjv".into()), 1, Vec::new())
		.unwrap();
	assert!(nothing.entries.is_empty());

	let found = source
		.get_search_manga_list(Some("hound".into()), 1, Vec::new())
		.unwrap();
	assert!(found.entries.iter().any(|manga| manga.key == SERIES));

	let adult = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "rating".into(),
				value: "only_18".into(),
			}],
		)
		.unwrap();
	assert!(!adult.entries.is_empty());
	assert!(
		adult
			.entries
			.iter()
			.all(|manga| manga.content_rating == ContentRating::NSFW)
	);

	let pasted = source
		.get_search_manga_list(
			Some(format!("https://mangaball.com/title-detail/{SERIES}")),
			1,
			Vec::new(),
		)
		.unwrap();
	assert_eq!(pasted.entries.len(), 1);
	assert_eq!(pasted.entries[0].key, SERIES);
	assert!(pasted.entries[0].description.is_none());
}

#[aidoku_test]
fn details_chapters_and_pages() {
	reset();
	let source = MangaBall::new();
	let manga = Manga {
		key: SERIES.into(),
		..Default::default()
	};
	let manga = source.get_manga_update(manga, true, true).unwrap();
	assert_eq!(manga.title, "Revenge of the Baskerville Bloodhound");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert!(manga.description.is_some() && manga.cover.is_some());
	assert!(
		manga
			.authors
			.as_ref()
			.unwrap()
			.contains(&"Seol Arang".to_string())
	);

	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() > 100);
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
	assert!(chapters[0].chapter_number >= chapters[chapters.len() - 1].chapter_number);

	let chapter = Chapter {
		key: chapters[chapters.len() - 1].key.clone(),
		..Default::default()
	};
	let pages = source
		.get_page_list(
			Manga {
				key: SERIES.into(),
				..Default::default()
			},
			chapter,
		)
		.unwrap();
	assert!(!pages.is_empty());

	defaults_set("languages", strings(&["id"]));
	let indonesian = source
		.get_manga_update(
			Manga {
				key: SERIES.into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert!(
		indonesian
			.chapters
			.unwrap()
			.iter()
			.all(|chapter| chapter.language.as_deref() == Some("id"))
	);
	reset();
}

#[aidoku_test]
fn unknown_series_fails() {
	let source = MangaBall::new();
	let manga = Manga {
		key: "000000000000000000000000".into(),
		..Default::default()
	};
	assert!(source.get_manga_update(manga, true, false).is_err());
}

#[aidoku_test]
fn listings_and_home() {
	reset();
	let source = MangaBall::new();
	for id in ["lastupdate", "created_at"] {
		let listing = Listing {
			id: id.into(),
			..Default::default()
		};
		let page = source.get_manga_list(listing, 1).unwrap();
		assert!(!page.entries.is_empty() && page.entries.len() <= 24, "{id}");
		assert!(page.has_next_page, "{id}");
	}
	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.map(|component| component.title.clone().unwrap())
		.collect();
	assert_eq!(
		titles,
		SECTIONS.map(|(_, name, _)| name.to_string()).to_vec()
	);
}

#[aidoku_test]
fn deep_links() {
	let source = MangaBall::new();
	let series = source
		.handle_deep_link(format!(
			"https://mangaball.com/title-detail/{SERIES}-revenge"
		))
		.unwrap();
	assert_eq!(series, Some(DeepLinkResult::Manga { key: SERIES.into() }));
	let chapter = source
		.handle_deep_link("https://mangaball.com/chapter-detail/6af082f2a91bdf88681d2971".into())
		.unwrap();
	assert_eq!(
		chapter,
		Some(DeepLinkResult::Chapter {
			manga_key: SERIES.into(),
			key: "6af082f2a91bdf88681d2971".into()
		})
	);
}

#[aidoku_test]
fn all_sort_directions_load() {
	reset();
	let source = MangaBall::new();
	for (index, sort) in SORTS.iter().enumerate() {
		let mut first_keys = Vec::new();
		for ascending in [false, true] {
			let page = source
				.get_search_manga_list(
					None,
					1,
					vec![FilterValue::Sort {
						id: "sort".into(),
						index: index as i32,
						ascending,
					}],
				)
				.unwrap_or_else(|error| panic!("{sort}, ascending={ascending}: {error:?}"));
			assert!(!page.entries.is_empty(), "{sort}, ascending={ascending}");
			assert!(page.has_next_page);
			assert!(
				page.entries
					.iter()
					.all(|manga| !manga.key.trim().is_empty() && !manga.title.trim().is_empty())
			);
			first_keys.push(page.entries[0].key.clone());
		}
		assert_ne!(first_keys[0], first_keys[1], "{sort}");
	}
}

#[aidoku_test]
fn review_cleared_languages_do_not_browse_every_language() {
	reset();
	defaults_set("languages", strings(&[]));
	assert!(settings::languages().is_empty());
	let page = SearchParams::new(None).page(1).unwrap();
	assert!(page.entries.is_empty() && !page.has_next_page);
	assert!(MangaBall::new().get_home().unwrap().components.is_empty());
	reset();
}

#[aidoku_test]
fn review_search_defaults_leave_explicit_overrides_selectable() {
	let filters: serde_json::Value =
		serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	for id in ["type", "rating"] {
		let filter = filters
			.as_array()
			.unwrap()
			.iter()
			.find(|filter| filter["id"] == id)
			.unwrap();
		assert_eq!(filter["ids"][0], "");
		assert_eq!(filter["options"][0], "Use Source Setting");
		assert!(filter.get("default").is_none());
	}
}

#[aidoku_test]
fn review_adult_tags_override_an_incorrect_safe_flag() {
	reset();
	let post: Title = serde_json::from_value(serde_json::json!({
		"id": SERIES, "name": "Example", "is18plus": false,
		"availableTranslatedLanguages": ["en"],
		"tags": [{"id": "adult", "name": "Hentai"}, {"id": "mature", "name": "Mature"}]
	}))
	.unwrap();
	assert_eq!(post.content_rating(), ContentRating::NSFW);
	assert!(!SearchParams::new(None).allows_rating(&post));
	assert!(!SearchParams::new(None).allows(&post));
	defaults_set("showAdult", DefaultValue::Bool(true));
	assert!(SearchParams::new(None).allows(&post));
	defaults_set("adultOnly", DefaultValue::Bool(true));
	assert!(SearchParams::new(None).allows(&post));
	reset();
}

#[aidoku_test]
fn review_home_skips_blocked_groups_before_picking_a_chapter() {
	reset();
	defaults_set("excludedScanlators", strings(&[" GROUP B "]));
	let mut post = title(SERIES, "ja", false);
	post.recent_chapters.push(
		serde_json::from_value(serde_json::json!({
			"id": "selected", "lang": "en", "number": 38, "group": {"name": "Group C"}
		}))
		.unwrap(),
	);
	let value = fill(
		"lastupdate",
		"Latest Chapter Updates",
		Layout::Chapters,
		vec![post],
		&SearchParams::new(None),
	);
	let HomeComponentValue::MangaChapterList { entries, .. } = value else {
		panic!("wrong layout");
	};
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].chapter.key, "selected");
	let chapter: ChapterItem = serde_json::from_value(serde_json::json!({
		"id": "blocked", "lang": "en", "group": {"name": "Group B"}
	}))
	.unwrap();
	assert!(
		chapter
			.into_chapter(
				&site_languages(vec!["en".into()]),
				&settings::excluded_scanlators()
			)
			.is_none()
	);
	reset();
}

#[aidoku_test]
fn review_numbered_prose_is_not_a_repeated_chapter_label() {
	let languages = site_languages(vec!["en".into()]);
	for name in [
		"Chapter 12",
		"Ch.12",
		"Vol.5 Ch.12",
		"12",
		"12 Heroes",
		"Chapter 12: A Promise",
	] {
		let chapter: ChapterItem = serde_json::from_value(serde_json::json!({
			"id": "chapter", "lang": "en", "number": 12, "volume": 5, "name": name
		}))
		.unwrap();
		let chapter = chapter.into_chapter(&languages, &[]).unwrap();
		assert_eq!(
			chapter.title.as_deref(),
			name.contains("Heroes")
				.then_some(name)
				.or_else(|| name.contains("Promise").then_some(name))
		);
	}
}

#[aidoku_test]
fn review_credits_and_tags_are_trimmed_and_unique() {
	let post: Title = serde_json::from_value(serde_json::json!({
		"id": SERIES, "name": "Example",
		"author": [{"name": " Writer "}, {"name": ""}, {"name": "Writer"}, {"name": "Studio"}],
		"tags": [{"id": "a", "name": "Drama"}, {"id": "b", "name": "Action"}, {"id": "c", "name": "Drama"}]
	})).unwrap();
	let details = post.into_details().unwrap();
	assert_eq!(
		details.authors,
		Some(vec!["Writer".into(), "Studio".into()])
	);
	assert_eq!(details.tags, Some(vec!["Action".into(), "Drama".into()]));
}

#[aidoku_test]
fn search_filters_override_settings() {
	reset();
	defaults_set("showAdult", DefaultValue::Bool(true));
	defaults_set("type", DefaultValue::String("manga".into()));
	let source = MangaBall::new();
	let overrides = vec![
		FilterValue::Select {
			id: "rating".into(),
			value: "no_18".into(),
		},
		FilterValue::Select {
			id: "type".into(),
			value: "any".into(),
		},
	];
	let page = source
		.get_search_manga_list(Some("Baskerville Bloodhound".into()), 1, overrides)
		.unwrap();
	assert!(page.entries.iter().any(|manga| manga.key == SERIES));
	assert!(
		page.entries
			.iter()
			.all(|manga| manga.content_rating != ContentRating::NSFW)
	);
	let inherited = source
		.get_search_manga_list(
			Some("Baskerville Bloodhound".into()),
			1,
			vec![FilterValue::Select {
				id: "type".into(),
				value: "".into(),
			}],
		)
		.unwrap();
	assert!(!inherited.entries.iter().any(|manga| manga.key == SERIES));
	reset();
}

#[aidoku_test]
fn safe_mode_hides_adult_tags_the_api_mislabels() {
	reset();
	let source = MangaBall::new();
	let filter = || {
		vec![FilterValue::MultiSelect {
			id: "theme".into(),
			included: vec!["68bfceaf4dbc442a26519889".into()],
			excluded: Vec::new(),
		}]
	};
	assert!(
		source
			.get_search_manga_list(None, 1, filter())
			.unwrap()
			.entries
			.is_empty()
	);
	defaults_set("showAdult", DefaultValue::Bool(true));
	assert!(
		!source
			.get_search_manga_list(None, 1, filter())
			.unwrap()
			.entries
			.is_empty()
	);
	reset();
}
