use super::{
	OniSaga,
	api::{BASE_URL, SearchParams},
	helpers::{chapter_number, chapters, details, gated, language_pages, next_page, terms},
	home::{SECTIONS, components},
	models::{Alternative, BrowsePage, GatedReader, GatedTitle, Rpc},
};
use aidoku::{
	ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, HomeComponentValue, Listing,
	ListingKind, ListingProvider, Manga, MangaStatus, Source, Viewer,
	alloc::{String, Vec, vec},
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
	},
};
use aidoku_test::aidoku_test;
use serde_json::Value;

// Captured shapes: the spotlight and latest-updates props from `self.__next_f`, then the shelves.
const HOME_HTML: &str = r#"<!doctype html><html><body>
<script>self.__next_f.push([1,"5:Te,Long synopsis.4:[\"$\",\"$L2d\",null,{\"titles\":[\"A\",\"B\"],\"thumbs\":[{\"id\":\"1\",\"slug\":\"safe-strip\",\"title\":\"Safe Strip\",\"origin\":\"manhwa\",\"layout\":\"vertical_strip\",\"publicationStatus\":\"completed\",\"synopsis\":\"Line one.\\n\\n(Source: Publisher)\",\"contentRating\":\"safe\",\"coverUrl\":\"https://media.onisaga.com/api/covers/safe-strip?mv=1\",\"languages\":[{\"language\":\"en\",\"readableUnits\":3}]},{\"id\":\"2\",\"slug\":\"teasing\",\"title\":\"Teasing\",\"origin\":\"manga\",\"publicationStatus\":\"releasing\",\"synopsis\":\"$5\",\"contentRating\":\"suggestive\",\"coverUrl\":\"https://media.onisaga.com/api/covers/teasing?mv=2\",\"languages\":[{\"language\":\"en\",\"readableUnits\":1}]}]}]\n37:[\"$\",\"div\",null,{\"children\":[[\"$\",\"$L46\",null,{\"entries\":[{\"card\":{\"id\":\"3\",\"slug\":\"french-only\",\"title\":\"French Only\",\"origin\":\"manhua\",\"contentRating\":\"safe\",\"languages\":[{\"language\":\"fr\",\"readableUnits\":5}]},\"units\":[{\"releaseId\":\"9\",\"label\":\"Chapter 5\",\"title\":null,\"language\":\"fr\",\"at\":\"2026-11-21T05:19:08.465626Z\"}]},{\"card\":{\"id\":\"4\",\"slug\":\"two-languages\",\"title\":\"Two Languages\",\"origin\":\"manhwa\",\"contentRating\":\"safe\",\"coverUrl\":\"https://media.onisaga.com/api/covers/two-languages?mv=4\",\"languages\":[{\"language\":\"fr\",\"readableUnits\":2},{\"language\":\"en\",\"readableUnits\":2}]},\"units\":[{\"releaseId\":\"20\",\"label\":\"Chapter 2\",\"title\":null,\"language\":\"fr\",\"at\":\"2026-11-21T05:19:08.465626Z\"},{\"releaseId\":\"21\",\"label\":\"Chapter 2\",\"title\":null,\"language\":\"en\",\"at\":\"2026-11-21T05:19:08.465626Z\"}]},{\"card\":\"$4:props:thumbs:0\",\"units\":[{\"releaseId\":\"30\",\"label\":\"Chapter 3\",\"title\":null,\"language\":\"en\",\"at\":null}]}],\"scope\":{\"kind\":\"default\"}}]]}]\n"])</script>
<section aria-labelledby="trending-title"><ol>
<li><span>1</span><a dir="auto" href="/title/blue-lock">Blue Lock</a></li>
<li><span>2</span><a dir="auto" href="/title/naruto">Naruto</a></li>
</ol></section>
<section aria-labelledby="finished-title"><ul>
<li><img src="https://media.onisaga.com/api/covers/slime?mv=82&amp;w=240"><span><span data-medium="manhua">Manhua</span></span><a href="/title/slime">Slime</a></li>
<li><span data-medium="manga">Manga</span><a href="/title/kaguya">Kaguya</a></li>
</ul></section>
</body></html>"#;

const SHELF_JSON: &str = r#"{"items":[{"id":"7","slug":"fire-ring","title":"Fire Ring","origin":null,"publicationStatus":"releasing","contentRating":"safe","coverUrl":"https://media.onisaga.com/api/covers/fire-ring?mv=22"},{"id":"8","slug":"untitled","title":null}],"nextCursor":"abc.def"}"#;

const TITLE_HTML: &str = r#"<!doctype html><html><body><main>
<section aria-labelledby="work-title">
<img class="banner" src="https://onisaga.com/api/covers/tsun?variant=banner&amp;w=1280">
<span data-cover=""><img src="https://media.onisaga.com/api/covers/tsun?mv=34&amp;w=240"></span>
<div><span data-medium="manhwa">Manhwa</span><span>Publication on hiatus</span><span><span>Suggestive</span></span><span><span>Korean → English</span></span></div>
<h1 id="work-title">Tsun-Yandere</h1>
<p><span>Story <a href="/creator/a">Writer One</a></span> <span>Art <a href="/creator/b">Artist One</a></span> <span>Story and art <a href="/creator/c">Both</a></span> <span>Story <a href="/creator/a">Writer One</a></span></p>
<div><a href="/browse?genre=romance">Romance</a><a href="/browse?genre=comedy">Comedy</a><a href="/browse?genre=comedy">Comedy</a></div>
</section>
<p id="synopsis">First line.
Second line.</p>
<section aria-labelledby="chapters-heading"><ol>
<li><a data-releases="105" data-unit="1" href="/read/tsun/105"><span id="unit-1-n"><span class="sr-only">Read. </span>Ch 37</span><span id="unit-1-t"><span class="sr-only">Chapter 37</span></span><time datetime="2026-11-21T05:19:08.465626Z">Nov 21</time></a></li>
<li class="langItem"><div data-releases="104" data-unit="2"><span id="unit-2-n">Ch 36.5</span><span id="unit-2-t"><span class="sr-only">Chapter 36.5</span>The Long Night</span><time datetime="2024-08-31T08:37:29.000000Z">Aug 31</time><a href="/read/tsun/104"></a></div>
<ul class="langPanel"><li><div><span>FR</span><a href="/group/fr-team">FR Team</a><a href="/read/tsun/999"></a></div></li></ul></li>
<li><details data-releases="102 103" data-unit="3"><summary><span id="unit-3-n">Ch 36</span><span id="unit-3-t"></span></summary>
<ul><li><div><em>No group credited</em><time datetime="2025-11-15T15:27:21.000000Z">x</time><a href="/read/tsun/102"></a></div></li>
<li><div><a href="/group/alpha-1">Alpha</a><time datetime="2025-10-07T14:08:34.000000Z">x</time><a href="/read/tsun/103"></a></div></li></ul></details></li>
<li><div data-releases="" data-unit="4"><span id="unit-4-n">Ch 35</span><span>Not available here</span></div></li>
</ol>
<nav aria-label="Chapter pages"><a href="/title/tsun?lang=en&amp;order=desc&amp;chapters=all&amp;cc=next.token">Next 100 chapters</a></nav>
</section>
<dl><div><dt>Status</dt><dd>Publication on hiatus</dd></div><div><dt>Layout</dt><dd>Vertical scroll</dd></div></dl>
</main></body></html>"#;

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

fn keys(entries: &[Manga]) -> Vec<&str> {
	entries.iter().map(|manga| manga.key.as_str()).collect()
}

#[aidoku_test]
fn review_home_order_layout_and_settings() {
	reset();
	let doc = Html::parse_with_url(HOME_HTML, BASE_URL).unwrap();
	let shelf = || serde_json::from_str::<BrowsePage>(SHELF_JSON).ok();
	let layout = components(&SearchParams::new(None), Some(&doc), shelf(), shelf());
	let titles = layout
		.iter()
		.map(|component| (component.title.as_deref(), component.subtitle.as_deref()))
		.collect::<Vec<_>>();
	assert_eq!(
		titles,
		[
			(Some("Spotlight"), None),
			(Some("Trending this week"), None),
			(
				Some("Start a new series"),
				Some("Recently added to OniSaga")
			),
			(Some("Latest updates"), None),
			(Some("Finished series"), None),
			(
				Some("Newly published"),
				Some("By original publication date")
			),
		]
	);
	assert_eq!(SECTIONS.len(), layout.len());

	let HomeComponentValue::BigScroller {
		entries,
		auto_scroll_interval,
	} = &layout[0].value
	else {
		panic!("spotlight is not a big scroller");
	};
	// Suggestive is off by default.
	assert_eq!(keys(entries), ["safe-strip"]);
	assert_eq!(*auto_scroll_interval, Some(8.0));
	let hero = &entries[0];
	assert_eq!(
		hero.description.as_deref(),
		Some("Line one.\n\n(Source: Publisher)")
	);
	assert_eq!(hero.viewer, Viewer::Webtoon);
	assert_eq!(hero.status, MangaStatus::Completed);

	let HomeComponentValue::MangaList {
		ranking,
		page_size,
		entries,
		..
	} = &layout[1].value
	else {
		panic!("trending is not a ranked list");
	};
	assert!(*ranking);
	assert_eq!(*page_size, Some(3));
	let trending = entries
		.iter()
		.map(|link| link.title.as_str())
		.collect::<Vec<_>>();
	assert_eq!(trending, ["Blue Lock", "Naruto"]);
	assert!(entries.iter().all(|link| link.subtitle.is_none()));

	let HomeComponentValue::Scroller { entries, listing } = &layout[2].value else {
		panic!("new series is not a scroller");
	};
	assert_eq!(entries.len(), 1, "an untitled card is dropped");
	assert_eq!(
		listing.as_ref().map(|listing| listing.id.as_str()),
		Some("new")
	);

	let HomeComponentValue::MangaChapterList {
		page_size, entries, ..
	} = &layout[3].value
	else {
		panic!("latest updates is not a chapter list");
	};
	assert_eq!(*page_size, Some(5));
	// English only: the French title goes, the bilingual one shows its English release, and a
	// card sent earlier as a spotlight slide arrives as a reference.
	let latest = entries
		.iter()
		.map(|entry| (entry.manga.key.as_str(), entry.chapter.key.as_str()))
		.collect::<Vec<_>>();
	assert_eq!(latest, [("two-languages", "21"), ("safe-strip", "30")]);
	assert_eq!(entries[0].chapter.key, "21");
	assert_eq!(entries[0].chapter.chapter_number, Some(2.0));
	assert_eq!(entries[0].chapter.language.as_deref(), Some("en"));
	assert!(entries[0].chapter.url.is_none());

	let HomeComponentValue::Scroller { entries, .. } = &layout[4].value else {
		panic!("finished is not a scroller");
	};
	let finished = entries
		.iter()
		.map(|link| link.title.as_str())
		.collect::<Vec<_>>();
	assert_eq!(finished, ["Slime", "Kaguya"]);

	defaults_set("origins", DefaultValue::StringArray(vec!["manga".into()]));
	defaults_set(
		"contentRatings",
		DefaultValue::StringArray(vec!["safe".into(), "suggestive".into()]),
	);
	let layout = components(&SearchParams::new(None), Some(&doc), None, None);
	let HomeComponentValue::BigScroller { entries, .. } = &layout[0].value else {
		panic!("spotlight is not a big scroller");
	};
	assert_eq!(keys(entries), ["teasing"]);
	// A text row that runs into the next row still resolves.
	assert_eq!(entries[0].description.as_deref(), Some("Long synopsis."));
	let HomeComponentValue::Scroller { entries, .. } = &layout[3].value else {
		panic!("finished is not a scroller");
	};
	let finished = entries
		.iter()
		.map(|link| link.title.as_str())
		.collect::<Vec<_>>();
	assert_eq!(finished, ["Kaguya"]);
	reset();
}

#[aidoku_test]
fn review_cleared_settings_browse_nothing() {
	reset();
	defaults_set("languages", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());
	reset();
	defaults_set("contentRatings", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());
	reset();
	defaults_set("origins", DefaultValue::StringArray(Vec::new()));
	assert!(SearchParams::new(None).is_empty());
	reset();
	assert!(!SearchParams::new(None).is_empty());
}

#[aidoku_test]
fn review_search_body_matches_the_site() {
	reset();
	defaults_set(
		"excludedGenres",
		DefaultValue::StringArray(vec!["horror".into(), "romance".into()]),
	);
	assert_eq!(
		body(&SearchParams::new(None)),
		serde_json::json!({"json": {
			"ratings": ["safe"],
			"languages": ["en"],
			"genres": {"include": [], "exclude": ["horror", "romance"]},
			"sort": "updated",
			"limit": 60,
		}})
	);
	assert_eq!(
		body(&SearchParams::new(Some("love".into())))["json"]["sort"],
		"relevance"
	);

	let mut search = SearchParams::new(Some("love".into()));
	search.origin = Some(vec!["manhwa".into()]);
	search.genres.include = vec!["romance".into()];
	terms(
		"Magic, -Military",
		&mut search.themes.include,
		&mut search.themes.exclude,
	);
	search.readable = true;
	let json = body(&search);
	assert_eq!(json["json"]["origin"], serde_json::json!(["manhwa"]));
	assert_eq!(
		json["json"]["themes"],
		serde_json::json!({"include": ["magic"], "exclude": ["military"]})
	);
	assert_eq!(json["json"]["readable"], true);
	assert!(json["json"].get("tags").is_none());
	reset();
}

#[aidoku_test]
fn review_terms_use_site_slugs() {
	let (mut include, mut exclude) = (Vec::new(), Vec::new());
	terms(
		" LGBTQ+ Themes , -Coming of Age,Boys' Love,, -",
		&mut include,
		&mut exclude,
	);
	assert_eq!(include, ["lgbtq-themes", "boys-love"]);
	assert_eq!(exclude, ["coming-of-age"]);
}

#[aidoku_test]
fn review_title_page_details_and_chapters() {
	let doc = Html::parse_with_url(TITLE_HTML, BASE_URL).unwrap();
	let manga = details(&doc, "tsun").unwrap();
	assert_eq!(manga.key, "tsun");
	assert_eq!(manga.title, "Tsun-Yandere");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://media.onisaga.com/api/covers/tsun?mv=34")
	);
	assert_eq!(
		manga.authors,
		Some(vec![String::from("Writer One"), String::from("Both")])
	);
	assert_eq!(
		manga.artists,
		Some(vec![String::from("Artist One"), String::from("Both")])
	);
	assert_eq!(
		manga.tags,
		Some(vec![String::from("Comedy"), String::from("Romance")])
	);
	assert_eq!(manga.status, MangaStatus::Hiatus);
	assert_eq!(manga.content_rating, ContentRating::Suggestive);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert!(
		manga
			.description
			.as_deref()
			.is_some_and(|text| text.contains("Second line"))
	);

	let mut list = Vec::new();
	chapters(&doc, "tsun", "en", &mut list);
	let rows = list
		.iter()
		.map(|chapter| {
			(
				chapter.key.as_str(),
				chapter.chapter_number,
				chapter.title.as_deref(),
				chapter.scanlators.clone(),
			)
		})
		.collect::<Vec<_>>();
	assert_eq!(
		rows,
		[
			("105", Some(37.0), None, None),
			("104", Some(36.5), Some("The Long Night"), None),
			("102", Some(36.0), None, None),
			("103", Some(36.0), None, Some(vec![String::from("Alpha")])),
		]
	);
	assert!(
		list.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
	assert_eq!(
		list[0].url.as_deref(),
		Some("https://onisaga.com/read/tsun/105")
	);
	assert_eq!(
		next_page(&doc).as_deref(),
		Some("https://onisaga.com/title/tsun?lang=en&order=desc&chapters=all&cc=next.token")
	);
}

#[aidoku_test]
fn review_adult_gates_and_api() {
	let gate = |html: &str| gated(&Html::parse(html).unwrap());
	assert!(!gate("<main><h1>Open</h1></main>").unwrap());
	assert!(gate(r#"<main data-ui="content-gate" data-gate="self-confirmation"></main>"#).unwrap());
	assert!(gate(r#"<main data-ui="content-gate" data-gate="account"></main>"#).is_err());

	let title: GatedTitle = serde_json::from_str(
		r#"{"title":"Night","synopsis":"","publicationStatus":"releasing","language":"en","units":[{"label":"Chapter 1","title":null,"releaseId":"11"},{"label":"Chapter 2","title":"Dawn","releaseId":"12"},{"label":"Chapter 3","title":null,"releaseId":null}]}"#,
	)
	.unwrap();
	let manga = title.into_manga("night", &[String::from("en")]);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert!(manga.description.is_none());
	let chapters = manga.chapters.unwrap();
	let keys = chapters
		.iter()
		.map(|chapter| (chapter.key.as_str(), chapter.chapter_number))
		.collect::<Vec<_>>();
	// Newest first, and a unit without a release is left out.
	assert_eq!(keys, [("12", Some(2.0)), ("11", Some(1.0))]);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://onisaga.com/read/night/12")
	);

	let reader: GatedReader = serde_json::from_str(
		r#"{"bootstrap":{"manifest":{"pages":[{"url":"https://media.onisaga.com/api/media/a?k=p"}]}}}"#,
	)
	.unwrap();
	assert_eq!(reader.into_urls(), ["https://onisaga.com/api/media/a?k=p"]);
}

#[aidoku_test]
fn review_language_pages_follow_the_site_links() {
	// Naruto's list: a main numbering, then a merged "other numbering" for some languages.
	let alternatives: Vec<Alternative> = serde_json::from_str(
		r#"[{"href":"/title/naruto?lang=en&order=desc","current":true},{"href":"/title/naruto?lang=ja&order=desc","current":false},{"href":"/title/naruto?stream=78117&lang=en&order=desc","current":false},{"href":"/title/naruto?stream=78117&lang=ja&order=desc","current":false},{"href":"/title/naruto?stream=78117&lang=th&order=desc","current":false}]"#,
	)
	.unwrap();
	let languages = ["ja", "en", "th", "fr"].map(String::from);
	assert_eq!(
		language_pages(&alternatives, &languages),
		[
			(
				"ja",
				Some(String::from(
					"https://onisaga.com/title/naruto?lang=ja&order=desc&chapters=all"
				))
			),
			("en", None),
			(
				"th",
				Some(String::from(
					"https://onisaga.com/title/naruto?stream=78117&lang=th&order=desc&chapters=all"
				))
			),
		]
	);
}

#[aidoku_test]
fn review_chapter_labels() {
	assert_eq!(chapter_number("Ch 700.5"), Some(700.5));
	assert_eq!(chapter_number("Chapter 46"), Some(46.0));
	assert_eq!(chapter_number("12"), Some(12.0));
	assert_eq!(chapter_number("Oneshot"), None);
}

#[aidoku_test]
fn review_deep_links() {
	let source = OniSaga::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert_eq!(
		link("https://onisaga.com/title/naruto?tab=details"),
		Some(DeepLinkResult::Manga {
			key: "naruto".into()
		})
	);
	assert_eq!(
		link("https://onisaga.com/read/naruto/100668105#page-1"),
		Some(DeepLinkResult::Chapter {
			manga_key: "naruto".into(),
			key: "100668105".into()
		})
	);
	assert_eq!(link("https://onisaga.com/browse?genre=action"), None);
}

#[aidoku_test]
fn review_resources() {
	let filters: Value = serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let filters = filters.as_array().unwrap();
	assert_eq!(filters[0]["type"], "sort");
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
	assert_eq!(ratings["default"], serde_json::json!(["safe"]));
	let all = serde_json::json!(["safe", "suggestive", "mature", "explicit"]);
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
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(!second.entries.is_empty());
	assert!(
		second
			.entries
			.iter()
			.all(|manga| !keys(&first.entries).contains(&manga.key.as_str()))
	);
}

#[aidoku_test]
fn every_sort_and_listing() {
	reset();
	let source = OniSaga::new();
	for index in 0..4 {
		let filters = vec![FilterValue::Sort {
			id: "sort".into(),
			index,
			ascending: false,
		}];
		let page = source.get_search_manga_list(None, 1, filters).unwrap();
		assert!(!page.entries.is_empty(), "sort {index}");
	}
	for id in ["new", "published"] {
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
			aidoku::Chapter {
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
