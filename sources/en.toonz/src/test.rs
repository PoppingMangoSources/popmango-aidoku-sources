#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, Home, HomeComponentValue, ListingKind, MangaStatus, Viewer,
	alloc::string::ToString,
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
	},
};
use aidoku_test::aidoku_test;
use helpers::{age, slugify};

const SERIES: &str = "solo-leveling";

const SERIES_PAGE: &str = r#"<html><head><link rel="canonical" href="https://toonz.to/manhwa/example"/>
<meta property="og:image" content="https://toonz.to/uploads/covers/example.webp"/></head><body><main>
<span hidden data-comic-id="1069693"></span><img data-cover data-nsfw src="/uploads/covers/example_thumb.webp"/>
<h1>Example Series</h1><dl><div><dt>Type</dt><dd>Manhwa</dd></div><div><dt>Status</dt><dd>Completed</dd></div>
<div><dt>Author</dt><dd><a href="/author/a">First Writer</a><a href="/author/b">Second</a><a href="/author/a">First Writer</a></dd></div></dl>
<p class="whitespace-pre-line">A *plain* summary.</p><div><p>Genres</p><a href="/genre/romance">Romance</a><a href="/genre/drama">Drama</a></div>
</main></body></html>"#;

const MANHWA_PAGE: &str = r#"<html><body><main>
<section aria-label="Featured"><article><p>Top rated this month</p><h2><a href="/manhwa/low">Low</a></h2><p>Low summary.</p>
<div><a href="/genre/romance">Romance</a></div><dl><div><dt>Rating</dt><dd><b>4.2</b> rating</dd></div></dl><img data-cover src="/low.webp"/></article>
<article><h2><a href="/manhwa/high">High</a></h2><p>High summary.</p><div><a href="/genre/smut">Smut</a><a href="/genre/drama">Drama</a></div>
<dl><div><dt>Rating</dt><dd><b>5.0</b> rating</dd></div></dl><img data-cover data-nsfw src="/high.webp"/></article></section>
<section><h2>Trending</h2><div role="tabpanel"><a href="/manhwa/today"><img data-cover src="/t.webp" alt="Today"/><h3>Today</h3><span>Ch. 4</span></a></div>
<div role="tabpanel" hidden><a href="/manhwa/week"><img data-cover src="/w.webp" alt="Week"/><h3>Week</h3></a></div></section>
<section><h2>New Titles</h2><a href="/manhwa/older"><img data-cover src="/o.webp"/><h3>Older</h3><span>Ch. 2.1</span><span>2h ago</span></a></section>
</main></body></html>"#;

const MANGA_PAGE: &str = r#"<html><body><main>
<section><h2>New Titles</h2><a href="/manga/newer"><img data-cover src="/n.webp"/><h3>Newer</h3><span>Ch. 1</span><span>5m ago</span></a>
<a href="/manga/older"><img data-cover src="/o.webp"/><h3>Older</h3><span>2h ago</span></a></section>
</main></body></html>"#;

const LATEST_PAGE: &str = r#"<html><body><main><article><a href="/manga/kawaisou"><div><img src="/k.webp" alt="Kawaisou"/></div></a>
<div><div><a href="/manga/kawaisou">Kawaisou</a></div><div><span><span>manga</span></span><span><span>·</span><span>CH.<!-- -->15.5</span></span></div>
<p>Summary.</p><p>2w ago</p></div></article></main></body></html>"#;

// The runner keeps a value that is set to `Null`, so each default is written out.
fn content(rating: &str, types: &[&str], excluded: &[&str]) {
	let list =
		|values: &[&str]| DefaultValue::StringArray(values.iter().map(|v| v.to_string()).collect());
	defaults_set("contentRating", DefaultValue::String(rating.into()));
	defaults_set("types", list(types));
	defaults_set("excludedGenres", list(excluded));
}

fn reset() {
	content(
		"erotica",
		&["manhwa", "manga", "manhua", "webtoon", "comic"],
		&[],
	);
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	Toonz::new()
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
fn review_home_rows_keep_the_requested_order() {
	let titles: Vec<_> = home::SECTIONS.iter().map(|(title, _)| *title).collect();
	assert_eq!(
		titles,
		[
			"Top Rated This Month",
			"New Titles",
			"Recently Updated",
			"Popular Updates (Manga)",
			"Popular Updates (Manhwa)",
			"Trending Manga",
			"Trending Manhwa",
		]
	);
}

#[aidoku_test]
fn review_home_rows_read_their_own_cards() {
	let pages = [
		Some(Html::parse_with_url(MANHWA_PAGE, BASE_URL).unwrap()),
		Some(Html::parse_with_url(MANGA_PAGE, BASE_URL).unwrap()),
		Some(Html::parse_with_url(LATEST_PAGE, BASE_URL).unwrap()),
	];
	let value = |index: usize| home::fill(home::SECTIONS[index].1, &pages);

	let Some(HomeComponentValue::BigScroller { entries, .. }) = value(0) else {
		panic!("top rated is not a big scroller");
	};
	let featured: Vec<_> = entries.iter().map(|manga| manga.key.as_str()).collect();
	assert_eq!(featured, ["high", "low"]);
	assert_eq!(
		entries[0].tags.as_deref(),
		Some(&["Drama".to_string(), "Smut".into()][..])
	);
	assert_eq!(entries[0].content_rating, ContentRating::NSFW);
	assert_eq!(entries[0].description.as_deref(), Some("High summary."));

	let Some(HomeComponentValue::Scroller { entries, .. }) = value(1) else {
		panic!("new titles is not a scroller");
	};
	let new: Vec<_> = entries.iter().map(|link| link.title.as_str()).collect();
	assert_eq!(new, ["Newer", "Older"]);

	let Some(HomeComponentValue::MangaChapterList {
		entries, page_size, ..
	}) = value(2)
	else {
		panic!("recently updated is not a chapter list");
	};
	assert_eq!(page_size, Some(5));
	assert_eq!(entries[0].manga.key, "kawaisou");
	assert_eq!(entries[0].chapter.key, "15.5");
	assert_eq!(entries[0].chapter.chapter_number, Some(15.5));
	assert!(entries[0].chapter.date_uploaded.is_some());

	let Some(HomeComponentValue::Scroller { entries, .. }) = value(6) else {
		panic!("trending manhwa is not a scroller");
	};
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].title, "Today");
	assert!(entries[0].subtitle.is_none());
	assert!(
		value(5).is_none(),
		"an empty row must not borrow another page's cards"
	);
}

#[aidoku_test]
fn review_details_read_the_series_page() {
	let page = Html::parse_with_url(SERIES_PAGE, BASE_URL).unwrap();
	let mut manga = Manga {
		key: "example".into(),
		..Default::default()
	};
	helpers::details(&page, &mut manga).unwrap();
	assert_eq!(manga.title, "Example Series");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://toonz.to/uploads/covers/example.webp")
	);
	assert_eq!(
		manga.url.as_deref(),
		Some("https://toonz.to/manhwa/example")
	);
	assert_eq!(
		manga.authors,
		Some(vec!["First Writer".into(), "Second".into()])
	);
	assert_eq!(manga.tags, Some(vec!["Drama".into(), "Romance".into()]));
	assert_eq!(manga.status, MangaStatus::Completed);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(manga.description.as_deref(), Some(r"A \*plain\* summary\."));
	assert_eq!(helpers::comic_id(&page).unwrap(), "1069693");
}

#[aidoku_test]
fn review_links_and_ages() {
	let link = |path: &str| helpers::series_path(path);
	assert_eq!(
		link("/manhwa/solo-leveling"),
		Some(("solo-leveling".into(), None))
	);
	assert_eq!(
		link("/manga/x/chapter/84.02?page=2"),
		Some(("x".into(), Some("84.02".into())))
	);
	assert_eq!(link("/manhwa/browse"), None);
	assert_eq!(link("/manga/trending"), None);
	assert_eq!(link("/genre/action"), None);
	assert_eq!(age("just now"), Some(0));
	assert_eq!(age("32m ago"), Some(32 * 60));
	assert_eq!(age("3mo ago"), Some(3 * 2_592_000));
	assert_eq!(age("Ch. 4"), None);
	for (name, slug) in [
		("Chugong", "chugong"),
		("13th month's dawn", "13th-months-dawn"),
		("2:10 AM Animation", "210-am-animation"),
		("3rd_R.I.O (써드리오)", "3rd-rio"),
		("1000+10 (atsuyu)", "100010-atsuyu"),
	] {
		assert_eq!(slugify(name), slug);
	}
}

#[aidoku_test]
fn review_cleared_types_match_nothing() {
	content("erotica", &[], &[]);
	assert_eq!(settings::types(), ["none"]);
	content("erotica", &["manga"], &[]);
	assert_eq!(settings::types(), ["manga"]);
	reset();
	assert!(settings::types().is_empty());
}

#[aidoku_test]
fn browse_pages_and_sorts() {
	reset();
	let first = search(None, Vec::new());
	assert_eq!(first.entries.len(), 24);
	assert!(first.has_next_page);
	let second = Toonz::new()
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
	content(
		"safe",
		&["manhwa", "manga", "manhua", "webtoon", "comic"],
		&[],
	);
	let safe = search(None, vec![sort(1)]);
	assert!(
		safe.entries
			.iter()
			.all(|manga| manga.content_rating != ContentRating::NSFW)
	);
	let explicit = search(None, vec![sort(1), select("rating", "pornographic")]);
	assert!(
		explicit
			.entries
			.iter()
			.any(|manga| manga.content_rating == ContentRating::NSFW)
	);
	reset();
}

#[aidoku_test]
fn settings_types_and_genres_apply_and_yield_to_search() {
	content("erotica", &[], &[]);
	assert!(search(None, Vec::new()).entries.is_empty());
	let override_types = search(None, vec![multi("type", &["manga"], &[])]);
	assert!(!override_types.entries.is_empty());

	content(
		"erotica",
		&["manhwa", "manga", "manhua", "webtoon", "comic"],
		&["romance"],
	);
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
	let layout = Toonz::new().get_home().unwrap();
	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	assert_eq!(titles, home::SECTIONS.map(|(title, _)| title));
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
	let listing = Toonz::new()
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
	let source = Toonz::new();
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
	let source = Toonz::new();
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
