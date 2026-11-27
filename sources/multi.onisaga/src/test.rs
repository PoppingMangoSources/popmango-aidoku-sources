use super::{
	OniSaga,
	api::{BASE_URL, CDN_URL, SORTS, SearchParams},
	helpers::{
		clean_chapter_title, media_url, page_expired, page_fragment, parse_chapter_number,
		parse_terms, plain_synopsis, slugify,
	},
	home::{SECTIONS, components},
	models::{
		Bootstrap, BrowsePage, ChapterPage, HomeData, Rpc, SelectInput, Selection, WorkDetail,
		Years,
	},
};
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, HomeComponentValue,
	Listing, ListingKind, ListingProvider, Manga, MangaStatus, Source, Viewer,
	alloc::{String, Vec, format, vec},
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use serde_json::{Value, json};

const HOME_JSON: &str = r#"{
	"spotlight": {
		"value": [
			{
				"card": {
					"slug": "safe-strip",
					"title": "Safe Strip",
					"medium": "comic",
					"origin": "manhwa",
					"layout": "vertical_strip",
					"publicationStatus": "completed",
					"synopsis": "**Bold** intro with [link](https://example.com).\n\n(Source: Publisher)",
					"contentRating": "safe",
					"coverUrl": "https://media.onisaga.com/api/covers/safe-strip?mv=1",
					"languages": [{"language": "en"}]
				}
			},
			{
				"card": {
					"slug": "teasing",
					"title": "Teasing",
					"medium": "comic",
					"origin": "manga",
					"publicationStatus": "releasing",
					"synopsis": "Suggestive slide.",
					"contentRating": "suggestive",
					"coverUrl": "https://media.onisaga.com/api/covers/teasing?mv=2",
					"languages": [{"language": "en"}]
				}
			},
			{
				"card": {
					"slug": "text-novel",
					"title": "Text Novel",
					"medium": "novel",
					"origin": null,
					"publicationStatus": "releasing",
					"synopsis": "Novel slide.",
					"contentRating": "safe",
					"coverUrl": "https://media.onisaga.com/api/covers/text-novel?mv=3",
					"languages": [{"language": "en"}]
				}
			}
		]
	},
	"trending": [
		{"slug": "blue-lock", "title": "Blue Lock", "medium": "comic", "origin": "manga", "contentRating": "safe"},
		{"slug": "adult-pick", "title": "Adult Pick", "medium": "comic", "origin": "manhwa", "contentRating": "mature"},
		{"slug": "prose-hit", "title": "Prose Hit", "medium": "novel", "origin": null, "contentRating": "safe"}
	],
	"newOnSite": {
		"value": [
			{
				"slug": "fire-ring",
				"title": "Fire Ring",
				"medium": "comic",
				"origin": "manhwa",
				"publicationStatus": "releasing",
				"contentRating": "safe",
				"coverUrl": "https://media.onisaga.com/api/covers/fire-ring?mv=22",
				"languages": [{"language": "en"}]
			},
			{
				"slug": "untitled",
				"title": null,
				"medium": "comic",
				"origin": "manga",
				"contentRating": "safe",
				"languages": [{"language": "en"}]
			}
		]
	},
	"latest": {
		"value": [
			{
				"card": {
					"slug": "french-only",
					"title": "French Only",
					"medium": "comic",
					"origin": "manhua",
					"contentRating": "safe",
					"languages": [{"language": "fr"}]
				},
				"units": [
					{"releaseId": "9", "label": "Chapter 5", "title": null, "language": "fr", "at": "2026-11-21T05:19:08.465626Z"}
				]
			},
			{
				"card": {
					"slug": "two-languages",
					"title": "Two Languages",
					"medium": "comic",
					"origin": "manhwa",
					"contentRating": "safe",
					"coverUrl": "https://media.onisaga.com/api/covers/two-languages?mv=4",
					"languages": [{"language": "fr"}, {"language": "en"}]
				},
				"units": [
					{"releaseId": "20", "label": "Chapter 2", "title": null, "language": "fr", "at": "2026-11-21T05:19:08.465626Z"},
					{"releaseId": "21", "label": "Season 2 · Episode 8", "title": "Chapter 8", "language": "en", "at": "2026-11-21T05:19:08.465626Z"}
				]
			}
		]
	},
	"finished": {
		"value": [
			{
				"slug": "slime",
				"title": "Slime",
				"medium": "comic",
				"origin": "manhua",
				"publicationStatus": "completed",
				"contentRating": "safe",
				"coverUrl": "https://media.onisaga.com/api/covers/slime?mv=82",
				"languages": [{"language": "en"}]
			}
		]
	},
	"newlyPublished": {
		"value": [
			{
				"slug": "early-work",
				"title": "Early Work",
				"medium": "comic",
				"origin": "manga",
				"publicationStatus": "releasing",
				"contentRating": "safe",
				"coverUrl": "https://media.onisaga.com/api/covers/early-work?mv=5",
				"languages": [{"language": "en"}]
			}
		]
	}
}"#;

const DETAIL_JSON: &str = r#"{
	"slug": "tsun",
	"canonical_title": "Tsun-Yandere",
	"medium": "comic",
	"origin": "manhwa",
	"layout": "vertical_strip",
	"demographic": "josei",
	"publication_status": "hiatus",
	"content_rating": "suggestive",
	"synopsis_md": "First line.\n\n*Second* line.",
	"metadata_version": "34",
	"credits": [
		{"name": "Writer One", "role": "story"},
		{"name": "Artist One", "role": "art"},
		{"name": "Both", "role": "story_and_art"},
		{"name": "Writer One", "role": "author"},
		{"name": "Helper", "role": "assistant"}
	],
	"terms": [
		{"group": "genre", "name": "Romance", "spoiler": false},
		{"group": "theme", "name": "Time Travel", "spoiler": false},
		{"group": "genre", "name": "Comedy", "spoiler": false},
		{"group": "tag", "name": "Secret Ending", "spoiler": true},
		{"group": "genre", "name": "Comedy", "spoiler": false}
	],
	"reading": {
		"workId": "900",
		"streams": [
			{"id": "10", "isDefault": true},
			{"id": "20", "isDefault": false}
		],
		"languages": [
			{"streamId": "20", "editionId": null, "language": "en", "readableUnits": 50},
			{"streamId": "10", "editionId": "5", "language": "en", "readableUnits": 37},
			{"streamId": "20", "editionId": null, "language": "th", "readableUnits": 12}
		],
		"groups": [
			{"id": "g1", "name": "Alpha"},
			{"id": "g2", "name": "Beta"}
		],
		"defaultScope": {"streamId": "10", "editionId": "5", "language": "en"}
	}
}"#;

fn reset() {
	defaults_set("languages", DefaultValue::StringArray(vec!["en".into()]));
	defaults_set(
		"contentRatings",
		DefaultValue::StringArray(vec!["safe".into()]),
	);
	defaults_set(
		"origins",
		DefaultValue::StringArray(
			[
				"manga",
				"manhwa",
				"manhua",
				"comic",
				"oel",
				"other_asian",
				"other",
			]
			.into_iter()
			.map(Into::into)
			.collect(),
		),
	);
	defaults_set("excludedGenres", DefaultValue::StringArray(Vec::new()));
}

fn body(search: &SearchParams) -> Value {
	serde_json::to_value(Rpc { json: search }).unwrap()
}

#[aidoku_test]
fn review_home_sections_order_and_cards() {
	reset();
	let search = SearchParams::new(None);
	let home: HomeData = serde_json::from_str(HOME_JSON).unwrap();
	let built = components(&search, Some(home), None, None, None);

	let headers = built
		.iter()
		.map(|comp| (comp.title.as_deref().unwrap(), comp.subtitle.as_deref()))
		.collect::<Vec<_>>();
	assert_eq!(headers, SECTIONS);

	let HomeComponentValue::BigScroller {
		entries: spotlight,
		auto_scroll_interval,
	} = &built[0].value
	else {
		panic!("expected big scroller");
	};
	assert_eq!(*auto_scroll_interval, Some(8.0));
	assert_eq!(spotlight.len(), 1);
	assert_eq!(spotlight[0].key, "safe-strip");
	assert_eq!(spotlight[0].viewer, Viewer::Webtoon);
	assert_eq!(
		spotlight[0].description.as_deref(),
		Some("Bold intro with link.")
	);

	let HomeComponentValue::MangaList {
		ranking,
		page_size,
		entries: trending,
		..
	} = &built[1].value
	else {
		panic!("expected ranked list");
	};
	assert!(*ranking);
	assert_eq!(*page_size, Some(3));
	assert_eq!(trending.len(), 1);
	assert_eq!(trending[0].title, "Blue Lock");
	assert_eq!(
		trending[0].image_url.as_deref(),
		Some("https://media.onisaga.com/api/covers/blue-lock")
	);
	assert!(trending[0].subtitle.is_none());

	let HomeComponentValue::Scroller {
		entries: new_entries,
		listing: new_listing,
	} = &built[2].value
	else {
		panic!("expected new scroller");
	};
	assert_eq!(new_entries.len(), 1);
	assert_eq!(new_entries[0].title, "Fire Ring");
	assert!(new_entries[0].subtitle.is_none());
	assert_eq!(new_listing.as_ref().unwrap().id, "new");

	let HomeComponentValue::MangaChapterList {
		page_size,
		entries: latest,
		..
	} = &built[3].value
	else {
		panic!("expected chapter list");
	};
	assert_eq!(*page_size, Some(5));
	assert_eq!(latest.len(), 1);
	assert_eq!(latest[0].manga.key, "two-languages");
	assert!(latest[0].manga.description.is_none());
	assert_eq!(latest[0].chapter.key, "21");
	assert_eq!(latest[0].chapter.chapter_number, Some(8.0));
	assert!(latest[0].chapter.title.is_none());
	assert!(latest[0].chapter.url.is_none());

	let HomeComponentValue::Scroller {
		entries: finished,
		listing: finished_listing,
	} = &built[4].value
	else {
		panic!("expected finished scroller");
	};
	assert_eq!(finished.len(), 1);
	assert_eq!(finished[0].title, "Slime");
	assert_eq!(finished_listing.as_ref().unwrap().id, "finished");

	let HomeComponentValue::Scroller {
		entries: published,
		listing: published_listing,
	} = &built[5].value
	else {
		panic!("expected published scroller");
	};
	assert_eq!(published.len(), 1);
	assert_eq!(published[0].title, "Early Work");
	assert_eq!(published_listing.as_ref().unwrap().id, "published");

	let override_page: BrowsePage = serde_json::from_str(
		r#"{"items":[{"slug":"override","title":"Override","medium":"comic","contentRating":"safe"}],"nextCursor":null}"#,
	)
	.unwrap();
	let narrowed = components(&search, None, Some(override_page), None, None);
	assert_eq!(narrowed.len(), 1);
	assert_eq!(narrowed[0].title.as_deref(), Some("Start a new series"));
}

#[aidoku_test]
fn review_search_filters_overrides_and_limits() {
	reset();
	defaults_set(
		"excludedGenres",
		DefaultValue::StringArray(vec!["horror".into(), "drama".into()]),
	);
	let default_search = SearchParams::new(None);
	let json = body(&default_search)["json"].clone();
	assert_eq!(json["medium"], json!(["comic"]));
	assert_eq!(json["ratings"], json!(["safe"]));
	assert_eq!(json["languages"], json!(["en"]));
	assert_eq!(json["sort"], "updated");
	assert!(json.get("origin").is_none());
	assert_eq!(
		json["genres"],
		json!({"include": [], "exclude": ["horror", "drama"]})
	);

	let mut query_search = SearchParams::new(Some("naruto".into()));
	query_search.sort = SORTS[0];
	query_search.finalize();
	assert_eq!(query_search.sort, "relevance");
	query_search.sort = SORTS[1];
	query_search.finalize();
	assert_eq!(query_search.sort, "updated");

	let mut filtered = SearchParams::new(None);
	filtered.origin = Some(vec!["manhwa".into()]);
	filtered.ratings = vec!["mature".into()];
	filtered.genres.include.push("horror".into());
	filtered.genres.exclude.push("tragedy".into());
	parse_terms(
		"Magic, Shōnen, -Military, Boys' Love",
		&mut filtered.themes.include,
		&mut filtered.themes.exclude,
	);
	filtered.year = Some(Years {
		from: Some(2020),
		to: None,
	});
	filtered.readable = true;
	for idx in 0..25 {
		filtered.tags.exclude.push(format!("tag-{idx}"));
	}
	for idx in 0..15 {
		filtered.languages.push(format!("l{idx}"));
	}
	filtered.finalize();
	assert!(filtered.adult());
	assert_eq!(filtered.languages.len(), 10);
	assert_eq!(filtered.tags.exclude.len(), 20);
	assert_eq!(filtered.genres.include, ["horror"]);
	assert_eq!(filtered.genres.exclude, ["drama", "tragedy"]);
	assert_eq!(filtered.themes.include, ["magic", "shonen", "boys-love"]);
	assert_eq!(filtered.themes.exclude, ["military"]);

	defaults_set("contentRatings", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());
	reset();
	defaults_set("languages", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());
	reset();
	defaults_set("origins", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());

	reset();
	let source = OniSaga::new();
	let bad_years = vec![FilterValue::Range {
		id: "year".into(),
		from: Some(2025.0),
		to: Some(2020.0),
	}];
	assert!(source.get_search_manga_list(None, 1, bad_years).is_err());
}

#[aidoku_test]
fn review_details_credits_tags_and_chapters() {
	let mut detail: WorkDetail = serde_json::from_str(DETAIL_JSON).unwrap();
	let scopes = detail
		.reading
		.language_scopes(&["th".into(), "en".into(), "fr".into()]);
	assert_eq!(scopes.len(), 2);
	assert_eq!(scopes[0].language, "th");
	assert_eq!(scopes[0].stream_id, "20");
	assert_eq!(scopes[1].language, "en");
	assert_eq!(scopes[1].stream_id, "10");

	let groups = core::mem::take(&mut detail.reading.groups);
	let manga = Manga::try_from(detail).unwrap();
	assert_eq!(manga.key, "tsun");
	assert_eq!(manga.title, "Tsun-Yandere");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://media.onisaga.com/api/covers/tsun?mv=34")
	);
	assert_eq!(
		manga.authors.as_deref(),
		Some(&[String::from("Writer One"), String::from("Both")][..])
	);
	assert_eq!(
		manga.artists.as_deref(),
		Some(&[String::from("Artist One"), String::from("Both")][..])
	);
	assert_eq!(
		manga.tags.as_deref(),
		Some(
			&[
				String::from("Comedy"),
				String::from("Josei"),
				String::from("Romance"),
				String::from("Time Travel"),
			][..]
		)
	);
	assert_eq!(
		manga.description.as_deref(),
		Some("First line.\n\n*Second* line.")
	);
	assert_eq!(manga.status, MangaStatus::Hiatus);
	assert_eq!(manga.content_rating, ContentRating::Suggestive);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert_eq!(manga.url.as_deref(), Some("https://onisaga.com/title/tsun"));

	let page: ChapterPage = serde_json::from_str(
		r#"{
			"items": [
				{
					"id": "u1",
					"number": "36.5",
					"label": "Chapter 36.5",
					"title": "100 Years Later",
					"volume": "4",
					"readableCandidateCount": 2,
					"selection": null
				}
			],
			"nextCursor": null
		}"#,
	)
	.unwrap();
	let fallback: Selection = serde_json::from_str(
		r#"{
			"selectedReleaseId": "103",
			"candidates": [
				{"id": "102", "groupIds": ["g1"], "contentKind": "images", "publishedAt": "2025-11-15T15:27:21.000000Z", "readableAt": null, "isReadable": true, "title": "Vol 4 - Chap 36.5", "label": "Chapter 36.5"},
				{"id": "103", "groupIds": ["g2"], "contentKind": "images", "publishedAt": "2025-10-07T14:08:34.000000Z", "readableAt": null, "isReadable": true, "title": null, "label": "Chapter 36.5"},
				{"id": "104", "groupIds": [], "contentKind": "text", "publishedAt": "2025-10-07T14:08:34.000000Z", "readableAt": null, "isReadable": true, "title": null, "label": "Chapter 36.5"}
			]
		}"#,
	)
	.unwrap();
	let unit = page.items.into_iter().next().unwrap();
	let chapters: Vec<Chapter> = unit.into_chapters("tsun", "en", &groups, Some(fallback));
	assert_eq!(chapters.len(), 2);
	assert_eq!(chapters[0].key, "103");
	assert_eq!(chapters[0].chapter_number, Some(36.5));
	assert_eq!(chapters[0].volume_number, Some(4.0));
	assert_eq!(chapters[0].title.as_deref(), Some("100 Years Later"));
	assert_eq!(
		chapters[0].scanlators.as_deref(),
		Some(&[String::from("Beta")][..])
	);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://onisaga.com/read/tsun/103")
	);
	assert_eq!(chapters[1].key, "102");
	assert!(chapters[1].title.is_none());

	let select_input = serde_json::to_value(SelectInput {
		work_id: "900",
		stream_id: "10",
		edition_id: None,
		chapter_ids: &[String::from("u1")],
		language: "en",
		preference_version: 0,
	})
	.unwrap();
	assert!(select_input.get("editionId").unwrap().is_null());
}

#[aidoku_test]
fn review_chapter_labels_and_slugs() {
	assert_eq!(parse_chapter_number("Ch 700.5"), Some(700.5));
	assert_eq!(parse_chapter_number("Chapter 46"), Some(46.0));
	assert_eq!(parse_chapter_number("Season 2 · Episode 12"), Some(12.0));
	assert_eq!(parse_chapter_number("12"), Some(12.0));
	assert_eq!(parse_chapter_number("Volume 2"), None);
	assert_eq!(parse_chapter_number("Oneshot"), None);

	assert_eq!(
		clean_chapter_title(Some("Vol. 2 - Chapter 12".into()), "Chapter 12", Some(12.0)),
		None
	);
	assert_eq!(
		clean_chapter_title(Some("100 Years Later".into()), "Chapter 12", Some(12.0)),
		Some("100 Years Later".into())
	);
	assert_eq!(
		clean_chapter_title(None, "Oneshot", None),
		Some("Oneshot".into())
	);
	assert_eq!(slugify("LGBTQ+ Themes"), "lgbtq-themes");
	assert_eq!(slugify("Girls' Love"), "girls-love");
	assert_eq!(slugify("Shōnen"), "shonen");
	assert_eq!(
		plain_synopsis("*Note:* [Official](https://x.test) line.\n(Source: Webtoons)"),
		Some("Note: Official line.".into())
	);
}

#[aidoku_test]
fn review_reader_media_urls_and_expiry() {
	let safe = format!("{CDN_URL}/api/media/a?k=p&r=10&p=sg&e=2000");
	assert_eq!(media_url(safe.clone()), safe);

	let gated = format!("{CDN_URL}/api/media/a?k=p&r=10&p=sc&e=2000");
	assert_eq!(
		media_url(gated),
		format!("{BASE_URL}/api/media/a?k=p&r=10&p=sc&e=2000")
	);

	let tagged = format!("{safe}#oni=10.2");
	assert_eq!(page_fragment(&tagged), Some((safe.as_str(), "10", 2)));
	assert!(!page_expired(&safe, 1900));
	assert!(page_expired(&safe, 1945));

	let bootstrap: Bootstrap = serde_json::from_str(
		r#"{"kind":"images","manifest":{"pages":[{"ord":2,"url":"https://media.onisaga.com/api/media/b?p=sc"},{"ord":1,"url":"https://media.onisaga.com/api/media/a?p=sg"}]}}"#,
	)
	.unwrap();
	let pages = bootstrap.into_pages().unwrap();
	assert_eq!(pages[0].ord, 1);
	assert_eq!(pages[0].url, "https://media.onisaga.com/api/media/a?p=sg");
	assert_eq!(pages[1].ord, 2);
	assert_eq!(pages[1].url, "https://onisaga.com/api/media/b?p=sc");

	let prose: Bootstrap = serde_json::from_str(r#"{"kind":"text","manifest":null}"#).unwrap();
	assert!(prose.into_pages().is_err());
}

#[aidoku_test]
fn review_deep_links() {
	let source = OniSaga::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert_eq!(
		link("https://onisaga.com/title/naruto/?tab=details"),
		Some(DeepLinkResult::Manga {
			key: "naruto".into()
		})
	);
	assert_eq!(
		link("https://onisaga.com/read/naruto/100668105/#page-1"),
		Some(DeepLinkResult::Chapter {
			manga_key: "naruto".into(),
			key: "100668105".into()
		})
	);
	assert_eq!(
		link("https://onisaga.com/read/naruto/continue?release=10"),
		None
	);
	assert_eq!(link("https://onisaga.com/browse?genre=action"), None);
}

#[aidoku_test]
fn review_resources() {
	let source_info: Value = serde_json::from_str(include_str!("../res/source.json")).unwrap();
	assert_eq!(source_info["info"]["contentRating"], 1);
	assert_eq!(source_info["config"]["supportsTagSearch"], true);
	assert_eq!(source_info["config"]["supportsAuthorSearch"], true);
	assert_eq!(source_info["config"]["supportsArtistSearch"], true);
	assert_eq!(
		source_info["info"]["languages"].as_array().unwrap().len(),
		24
	);

	let filters: Value = serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let filters = filters.as_array().unwrap();
	assert_eq!(filters[0]["type"], "sort");
	assert!(filters[0].get("title").is_none());
	assert_eq!(filters[0]["options"].as_array().unwrap().len(), SORTS.len());
	assert!(filters.iter().all(|filter| filter["id"] != "language"));
	for filter in filters
		.iter()
		.filter(|filter| filter["type"] == "multi-select")
	{
		assert_eq!(
			filter["canExclude"],
			filter["id"] == "genre",
			"{} exclusion",
			filter["id"]
		);
	}
	let year = filters
		.iter()
		.find(|filter| filter["id"] == "year")
		.unwrap();
	assert_eq!(year["type"], "range");
	assert_eq!(year["min"], 1900);
	let readable = filters
		.iter()
		.find(|filter| filter["id"] == "readable")
		.unwrap();
	assert_eq!(readable["hideFromHeader"], true);

	let settings: Value = serde_json::from_str(include_str!("../res/settings.json")).unwrap();
	let ratings = settings[0]["items"]
		.as_array()
		.unwrap()
		.iter()
		.find(|item| item["key"] == "contentRatings")
		.unwrap();
	assert_eq!(ratings["default"], json!(["safe"]));
	let all = json!(["safe", "suggestive", "mature", "explicit"]);
	assert_eq!(ratings["values"], all);
	let rating = filters
		.iter()
		.find(|filter| filter["id"] == "rating")
		.unwrap();
	assert_eq!(rating["ids"], all);
}

#[aidoku_test]
fn browse_pages_with_cursor() {
	reset();
	let source = OniSaga::new();
	let first = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	assert!(first.has_next_page);
	assert!(!first.entries.is_empty());
	let first_keys = first
		.entries
		.iter()
		.map(|manga| manga.key.as_str())
		.collect::<Vec<_>>();
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(!second.entries.is_empty());
	assert!(
		second
			.entries
			.iter()
			.all(|manga| !first_keys.contains(&manga.key.as_str()))
	);
}

#[aidoku_test]
fn every_sort_and_listing() {
	reset();
	let source = OniSaga::new();
	for index in 0..SORTS.len() as i32 {
		let filters = vec![FilterValue::Sort {
			id: "sort".into(),
			index,
			ascending: false,
		}];
		let page = source.get_search_manga_list(None, 1, filters).unwrap();
		assert!(!page.entries.is_empty(), "sort {index}");
	}
	for id in ["updated", "new", "finished", "published"] {
		let listing = Listing {
			id: id.into(),
			name: id.into(),
			kind: ListingKind::Default,
		};
		assert!(
			!source
				.get_manga_list(listing, 1)
				.unwrap()
				.entries
				.is_empty()
		);
	}
}

#[aidoku_test]
fn rating_filter_overrides_setting() {
	reset();
	let source = OniSaga::new();
	let filters = vec![FilterValue::MultiSelect {
		id: "rating".into(),
		included: vec!["suggestive".into()],
		excluded: Vec::new(),
	}];
	let page = source.get_search_manga_list(None, 1, filters).unwrap();
	assert!(!page.entries.is_empty());
	assert!(
		page.entries
			.iter()
			.all(|manga| manga.content_rating == ContentRating::Suggestive)
	);
}

#[aidoku_test]
fn details_chapters_and_pages() {
	reset();
	let source = OniSaga::new();
	let manga = source
		.get_manga_update(
			Manga {
				key: "raising-a-child-who-will-kill-me".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "Nareul Jugil Aireul Kiunda");
	assert_eq!(manga.status, MangaStatus::Completed);
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() >= 37);
	let chapter = chapters.into_iter().last().unwrap();
	let pages = source
		.get_page_list(
			Manga {
				key: "raising-a-child-who-will-kill-me".into(),
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
fn unknown_title_fails() {
	reset();
	let source = OniSaga::new();
	let manga = Manga {
		key: "no-such-title-here".into(),
		..Default::default()
	};
	assert!(source.get_manga_update(manga, true, false).is_err());
}
