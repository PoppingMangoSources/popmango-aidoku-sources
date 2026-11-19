#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, Home, HomeComponentValue, MangaStatus, Viewer,
	alloc::string::ToString,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use models::{ChapterItem, Title};

const SERIES: &str = "solo-leveling";
const NOVEL: &str = "novel/lord-of-mysteries-2-circle-of-inevitability";
const TYPES: [&str; 16] = [
	"Artbook",
	"Comic",
	"Doujinshi",
	"Elenty",
	"French",
	"Indonesian",
	"Kayden",
	"Manga",
	"Manhua",
	"Manhwa",
	"Nordic",
	"Novel",
	"Oel",
	"Spanish",
	"Thai",
	"Vietnamese",
];

const UPLOADS: &str = r#"[
	{"id":"a1","lang":"en","number":"2","title":"Chapter 2","rank":"b","source_name":"groupa2","published_date":"2026-01-02"},
	{"id":"a2","lang":"en","number":"2","title":"Chapter 2","rank":"a","source_name":"groupa","published_date":"2026-01-02"},
	{"id":"a3","lang":"en","number":"2","title":"Chapter 2","rank":"c","source_name":"groupb","published_date":"0001-01-01"},
	{"id":"a4","lang":"fr","number":"2","title":"Chapter 2","rank":"a","source_name":"groupc"},
	{"id":"a5","lang":"en","number":"1","title":"The Start","rank":"a","source_name":"groupb"}
]"#;

fn strings(values: &[&str]) -> DefaultValue {
	DefaultValue::StringArray(values.iter().map(|value| value.to_string()).collect())
}

// The runner keeps a value that is set to `Null`, so each default is written out.
fn reset() {
	defaults_set("languages", strings(&["en"]));
	defaults_set("contentRating", DefaultValue::String("safe".into()));
	defaults_set("types", strings(&TYPES));
	defaults_set("excludedGenres", strings(&[]));
	defaults_set("excludedTags", strings(&[]));
	defaults_set("deduplicateChapters", DefaultValue::String("off".into()));
	defaults_set("preferredUploaders", strings(&[]));
	defaults_set("blockedUploaders", strings(&[]));
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	Swarm::new()
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

fn multi(id: &str, included: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| value.to_string()).collect(),
		excluded: Vec::new(),
	}
}

fn uploads(mode: &str, preferred: &[&str], blocked: &[&str], languages: &[&str]) -> Vec<String> {
	let items: Vec<ChapterItem> = serde_json::from_str(UPLOADS).unwrap();
	let owned = |values: &[&str]| {
		values
			.iter()
			.map(|value| value.to_string())
			.collect::<Vec<_>>()
	};
	chapter_list(
		items,
		"x",
		&owned(languages),
		&owned(blocked),
		mode,
		&owned(preferred),
	)
	.into_iter()
	.map(|chapter| chapter.key)
	.collect()
}

#[aidoku_test]
fn review_home_rows_keep_the_requested_order() {
	let titles: Vec<_> = home::SECTIONS.iter().map(|(title, _)| *title).collect();
	assert_eq!(
		titles,
		[
			"Most Recent Popular",
			"Hot Updates",
			"New Updates",
			"Most Followed New Comics",
			"Novels",
		]
	);
}

#[aidoku_test]
fn review_chapter_list_modes() {
	assert_eq!(uploads("off", &[], &[], &["en"]), ["a1", "a2", "a3", "a5"]);
	// The lowest rank letter wins within an uploader's copies.
	assert_eq!(uploads("uploader", &[], &[], &["en"]), ["a2", "a3", "a5"]);
	assert_eq!(uploads("chapter", &[], &[], &["en"]), ["a2", "a5"]);
	// Settings hand over names lowercased.
	assert_eq!(uploads("chapter", &["groupb"], &[], &["en"]), ["a3", "a5"]);
	assert_eq!(uploads("off", &[], &["groupa"], &["en"]), ["a3", "a5"]);
	assert_eq!(
		uploads("chapter", &[], &[], &["en", "fr"]),
		["a2", "a4", "a5"]
	);

	let items: Vec<ChapterItem> = serde_json::from_str(UPLOADS).unwrap();
	let chapters = chapter_list(items, "x", &["en".into()], &[], "off", &[]);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://swarm.ws/comic/x/chapter/2?b")
	);
	assert_eq!(
		chapters[1].url.as_deref(),
		Some("https://swarm.ws/comic/x/chapter/2")
	);
	assert_eq!(chapters[0].title, None);
	assert_eq!(chapters[3].title.as_deref(), Some("The Start"));
	assert_eq!(chapters[2].date_uploaded, None);
	assert_eq!(chapters[0].scanlators, Some(vec!["groupa2".into()]));
	assert_eq!(chapters[0].language.as_deref(), Some("en"));
}

#[aidoku_test]
fn review_titles_read_lists_in_either_shape() {
	let title: Title = serde_json::from_str(
		r#"{"title":"T","url":"t","genres":"['Action', \"Boys' Love\"]","tags":["Long Strip"],
		"type":"Manhwa","status":"Completed","authors":["A"],"nsfw":false,"cover":"https://cum.swarm.ws/c.jpg"}"#,
	)
	.unwrap();
	assert_eq!(title.genres, ["Action", "Boys' Love"]);
	let manga = title.into_details().unwrap();
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://swarm.ws/covers/c.jpg")
	);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert_eq!(manga.status, MangaStatus::Completed);
	assert_eq!(
		manga.tags,
		Some(vec![
			"Action".into(),
			"Boys' Love".into(),
			"Long Strip".into()
		])
	);
	let adult: Title =
		serde_json::from_str(r#"{"title":"T","url":"t","genres":["Smut"]}"#).unwrap();
	assert_eq!(adult.content_rating(), ContentRating::NSFW);
	let untitled: Title = serde_json::from_str(r#"{"title":" ","url":"t"}"#).unwrap();
	assert!(Manga::try_from(untitled).is_err());
}

#[aidoku_test]
fn review_links_covers_and_languages() {
	assert!(matches!(
		helpers::parse_link("https://swarm.ws/comic/solo-leveling"),
		Some(helpers::Link::Comic(slug)) if slug == SERIES
	));
	assert!(matches!(
		helpers::parse_link("https://swarm.ws/comic/x/chapter/12?c"),
		Some(helpers::Link::Chapter(slug, number, rank)) if slug == "x" && number == "12" && rank == "c"
	));
	assert!(matches!(
		helpers::parse_link("https://swarm.ws/comic/x/chapter/12"),
		Some(helpers::Link::Chapter(_, _, rank)) if rank == "a"
	));
	assert!(matches!(
		helpers::parse_link("https://swarm.ws/novel/y/chapter/3"),
		Some(helpers::Link::NovelChapter(slug, number)) if slug == "y" && number == "3"
	));
	assert!(helpers::parse_link("https://swarm.ws/browse").is_none());
	assert_eq!(
		helpers::cover_url("https://example.com/a b.jpg").as_deref(),
		Some("https://swarm.ws/api/image-proxy?url=https%3A%2F%2Fexample.com%2Fa%20b.jpg")
	);
	assert_eq!(helpers::app_language("es-la"), "es-419");
	assert_eq!(helpers::app_language("kr"), "ko");
	assert_eq!(helpers::app_language("pt-br"), "pt-BR");
	assert_eq!(helpers::app_language("fr"), "fr");
}

#[aidoku_test]
fn review_payload_is_iv_and_ciphertext() {
	let payload = helpers::encrypt_payload(r#"{"chapterId":"abc"}"#);
	let (iv, cipher) = payload.split_once(':').unwrap();
	assert_eq!(iv.len(), 32);
	assert_eq!(cipher.len(), 64);
	assert!(
		payload
			.bytes()
			.all(|byte| byte == b':' || byte.is_ascii_hexdigit())
	);
}

#[aidoku_test]
fn browse_pages_and_sorts() {
	reset();
	let first = search(None, Vec::new());
	assert!(!first.entries.is_empty());
	assert!(first.has_next_page);
	let second = Swarm::new()
		.get_search_manga_list(None, 2, Vec::new())
		.unwrap();
	assert_ne!(keys(&first), keys(&second));
	let title = keys(&search(
		Some("love"),
		vec![FilterValue::Sort {
			id: "sort".into(),
			index: 1,
			ascending: false,
		}],
	));
	let created = keys(&search(
		Some("love"),
		vec![FilterValue::Sort {
			id: "sort".into(),
			index: 2,
			ascending: false,
		}],
	));
	assert!(!title.is_empty() && !created.is_empty());
	assert_ne!(title, created);
}

#[aidoku_test]
fn search_finds_a_series_and_a_pasted_link() {
	reset();
	assert!(keys(&search(Some("solo leveling"), Vec::new())).contains(&SERIES.into()));
	let pasted = search(Some("https://swarm.ws/comic/solo-leveling"), Vec::new());
	assert_eq!(keys(&pasted), [SERIES]);
	assert!(pasted.entries[0].description.is_none());
}

#[aidoku_test]
fn filters_narrow_the_catalogue() {
	reset();
	let all = keys(&search(None, Vec::new()));
	for filter in [
		multi("genre", &["Romance"]),
		multi("type", &["Manga"]),
		FilterValue::Text {
			id: "tags".into(),
			value: "Isekai".into(),
		},
		FilterValue::Range {
			id: "year".into(),
			from: Some(2024.0),
			to: None,
		},
		FilterValue::Range {
			id: "chapters".into(),
			from: Some(500.0),
			to: None,
		},
	] {
		let narrowed = keys(&search(None, vec![filter]));
		assert!(!narrowed.is_empty());
		assert_ne!(narrowed, all);
	}
	let capped = search(
		None,
		vec![FilterValue::Range {
			id: "chapters".into(),
			from: None,
			to: Some(3.0),
		}],
	);
	assert!(capped.entries.len() < 50);
}

#[aidoku_test]
fn ratings_follow_the_setting_and_filter() {
	reset();
	let safe = search(None, Vec::new());
	assert!(
		safe.entries
			.iter()
			.all(|manga| manga.content_rating == ContentRating::Safe)
	);
	let nsfw = search(None, vec![multi("rating", &["nsfw"])]);
	assert!(!nsfw.entries.is_empty());
	assert!(
		nsfw.entries
			.iter()
			.all(|manga| manga.content_rating == ContentRating::NSFW)
	);
	// Both ratings are the site's "All".
	let all = search(None, vec![multi("rating", &["safe", "nsfw"])]);
	assert!(
		all.entries
			.iter()
			.any(|manga| manga.content_rating != ContentRating::Safe)
	);
	defaults_set("languages", strings(&[]));
	assert!(search(None, Vec::new()).entries.is_empty());
	assert!(Swarm::new().get_home().unwrap().components.is_empty());
	reset();
}

#[aidoku_test]
fn settings_apply_and_yield_to_search() {
	reset();
	defaults_set("types", strings(&[]));
	assert!(search(None, Vec::new()).entries.is_empty());
	assert!(
		!search(None, vec![multi("type", &["Manhwa"])])
			.entries
			.is_empty()
	);
	reset();
	defaults_set("excludedGenres", strings(&["Romance"]));
	let page = search(None, Vec::new());
	assert!(!page.entries.is_empty());
	let included = search(None, vec![multi("genre", &["Romance"])]);
	assert!(
		!included.entries.is_empty(),
		"an included genre must override its exclusion"
	);
	reset();
}

#[aidoku_test]
fn details_and_chapters_follow_languages_and_deduplication() {
	reset();
	let source = Swarm::new();
	let update = |languages: &[&str], mode: &str| {
		defaults_set("languages", strings(languages));
		defaults_set("deduplicateChapters", DefaultValue::String(mode.into()));
		source
			.get_manga_update(
				Manga {
					key: SERIES.into(),
					..Default::default()
				},
				true,
				true,
			)
			.unwrap()
	};
	let manga = update(&["en"], "off");
	assert_eq!(manga.title, "Solo Leveling");
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert!(manga.authors.is_some() && manga.description.is_some());
	let all = manga.chapters.unwrap();
	assert!(
		all.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
	let per_chapter = update(&["en"], "chapter").chapters.unwrap();
	assert!(per_chapter.len() < all.len());
	let both = update(&["en", "fr"], "chapter").chapters.unwrap();
	assert!(both.len() > per_chapter.len());
	reset();
	let chapter = per_chapter.into_iter().next().unwrap();
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
fn novels_read_from_keys_only() {
	reset();
	let source = Swarm::new();
	let novel = source
		.get_manga_update(
			Manga {
				key: NOVEL.into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(novel.title, "LORD OF MYSTERIES 2: CIRCLE OF INEVITABILITY");
	assert_eq!(
		novel.authors,
		Some(vec!["Cuttlefish That Loves Diving".into()])
	);
	let chapters = novel.chapters.unwrap();
	assert!(chapters.len() > 100);
	let pages = source
		.get_page_list(
			Manga {
				key: NOVEL.into(),
				..Default::default()
			},
			Chapter {
				key: "1".into(),
				..Default::default()
			},
		)
		.unwrap();
	let PageContent::Text(text) = &pages[0].content else {
		panic!("a novel chapter is not text");
	};
	assert!(text.contains("\n\n"));
}

#[aidoku_test]
fn home_loads_every_row() {
	reset();
	let layout = Swarm::new().get_home().unwrap();
	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(titles, home::SECTIONS.map(|(title, _)| title));
	for component in layout.components {
		match component.value {
			HomeComponentValue::Scroller { entries, .. } => {
				assert!(!entries.is_empty());
				assert!(entries.iter().all(|link| link.subtitle.is_none()));
			}
			HomeComponentValue::MangaChapterList { entries, .. } => {
				assert!(!entries.is_empty());
				assert!(
					entries
						.iter()
						.all(|entry| entry.chapter.chapter_number.is_some())
				);
			}
			_ => panic!("unexpected component"),
		}
	}
}

#[aidoku_test]
fn deep_links() {
	let source = Swarm::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert!(matches!(
		link("https://swarm.ws/comic/solo-leveling"),
		Some(DeepLinkResult::Manga { key }) if key == SERIES
	));
	assert!(matches!(
		link("https://swarm.ws/comic/solo-leveling/chapter/1"),
		Some(DeepLinkResult::Chapter { manga_key, .. }) if manga_key == SERIES
	));
	assert!(matches!(
		link("https://swarm.ws/novel/lord-of-mysteries-2-circle-of-inevitability"),
		Some(DeepLinkResult::Manga { key }) if key == NOVEL
	));
}
