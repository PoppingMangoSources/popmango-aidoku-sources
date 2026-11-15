use crate::{
	api::SearchParams,
	helpers::{credits, date, link_keys, tags},
	home::{self, Section},
	markdown,
	models::{ChapterItem, Title, Updates},
};
use aidoku::{
	Chapter, FilterValue, HomeComponentValue, Manga,
	alloc::{Vec, vec},
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
	},
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_home_component_kinds_and_sizes() {
	let blocked = aidoku::alloc::collections::BTreeSet::new();
	for section in [
		Section::Featured,
		Section::Ranking,
		Section::Recent,
		Section::PopularChapters,
		Section::Updates,
		Section::Followed,
		Section::Popular,
		Section::Novels,
		Section::LatestNovels,
		Section::HighScore,
	] {
		let data = match section {
			Section::Featured => home::Data::Featured(Vec::new()),
			Section::Novels => home::Data::Chapters(Vec::new()),
			_ => home::Data::Titles(Vec::new()),
		};
		let component = home::component(section, data, &[], &blocked);
		match section {
			Section::Featured => assert!(matches!(
				component.value,
				HomeComponentValue::BigScroller { .. }
			)),
			Section::Ranking => assert!(matches!(
				component.value,
				HomeComponentValue::MangaList {
					ranking: true,
					page_size: Some(3),
					..
				}
			)),
			Section::Updates | Section::Novels => assert!(matches!(
				component.value,
				HomeComponentValue::MangaChapterList {
					page_size: Some(4),
					..
				}
			)),
			_ => assert!(matches!(
				component.value,
				HomeComponentValue::Scroller { .. }
			)),
		}
	}
}

#[aidoku_test]
fn review_recently_added_uses_home_cards_in_site_order() {
	let html = Html::parse(r#"<div id="manga-grid"><div class="group"><a href="https://example.com/manga/first"><img src="first.jpg" alt="First"></a><a href="https://example.com/read/first/ch2-3">Ch. 2</a></div><div class="group"><a href="https://example.com/manga/second"><img src="second.jpg" alt="Second"></a></div></div><div class="group"><img alt="Unrelated" src="other.jpg"></div>"#).unwrap();
	let titles = home::parse_recent(&html)
		.into_iter()
		.map(|t| Manga::try_from(t).unwrap())
		.collect::<Vec<_>>();
	assert_eq!(
		titles.iter().map(|t| t.key.as_str()).collect::<Vec<_>>(),
		["first", "second"]
	);
	assert!(
		titles
			.iter()
			.all(|t| t.description.is_none() && t.authors.is_none() && t.tags.is_none())
	);
}

#[aidoku_test]
fn review_settings_search_overrides_and_clearing() {
	defaults_set("excludedTags", DefaultValue::StringArray(vec!["7".into()]));
	defaults_set(
		"excludedTypes",
		DefaultValue::StringArray(vec!["Novel".into()]),
	);
	let baseline = SearchParams::new(None, 1, Vec::new()).unwrap();
	assert_eq!(baseline.excluded_tags, ["7"]);
	assert_eq!(baseline.excluded_types, ["Novel"]);
	let search = SearchParams::new(
		Some("one & two".into()),
		2,
		vec![
			FilterValue::MultiSelect {
				id: "genre".into(),
				included: vec!["7".into()],
				excluded: vec!["20".into()],
			},
			FilterValue::MultiSelect {
				id: "type".into(),
				included: vec!["Novel".into()],
				excluded: Vec::new(),
			},
			FilterValue::Sort {
				id: "sort".into(),
				index: 2,
				ascending: true,
			},
		],
	)
	.unwrap();
	assert_eq!(search.genres, "[7]");
	assert_eq!(search.types, "[\"Novel\"]");
	assert_eq!(search.sort, "title_asc");
	// Settings exclusions stay, unless the search includes that tag or type
	assert_eq!(search.excluded_tags, ["20"]);
	assert!(search.excluded_types.is_empty());
	let extended = SearchParams::new(
		None,
		1,
		vec![FilterValue::MultiSelect {
			id: "genre".into(),
			included: Vec::new(),
			excluded: vec!["20".into()],
		}],
	)
	.unwrap();
	assert_eq!(extended.excluded_tags, ["7", "20"]);
	assert_eq!(extended.excluded_types, ["Novel"]);
	let clear = SearchParams::new(
		None,
		1,
		vec![FilterValue::MultiSelect {
			id: "genre".into(),
			included: Vec::new(),
			excluded: Vec::new(),
		}],
	)
	.unwrap();
	assert_eq!(clear.genres, "[]");
	assert_eq!(clear.excluded_tags, ["7"]);
	defaults_set("excludedTags", DefaultValue::StringArray(Vec::new()));
	defaults_set("excludedTypes", DefaultValue::StringArray(Vec::new()));
	let clear = SearchParams::new(None, 1, Vec::new()).unwrap();
	assert!(clear.excluded_tags.is_empty() && clear.excluded_types.is_empty());
}

#[aidoku_test]
fn review_update_uses_actual_uploaded_chapter() {
	let feed: Updates = serde_json::from_str(include_str!("fixtures/updates.json")).unwrap();
	let update = feed.data.into_iter().next().unwrap().into_update().unwrap();
	assert_eq!(update.chapter.chapter_number, Some(5.0));
	assert!(update.chapter.key.ends_with("ch5-709368"));
	assert!(update.chapter.date_uploaded.is_some());
	assert!(
		update.manga.authors.is_none()
			&& update.manga.description.is_none()
			&& update.manga.tags.is_none()
	);
}

#[aidoku_test]
fn review_keys_agree_across_chapters_deep_links_and_cards() {
	let item: ChapterItem = serde_json::from_str(r#"{"chapter":"3.5","title":"N/A","date":"2 hours ago","group_name":null,"language":"en","url":"https://example.com/read/a-title/ch3.5-45/"}"#).unwrap();
	let chapter = Chapter::try_from(item).unwrap();
	let (manga_key, deep_chapter) =
		link_keys("https://example.com/read/a-title/ch3.5-45/?utm=1").unwrap();
	assert_eq!(chapter.key, deep_chapter.unwrap());
	assert_eq!(manga_key, "a-title");
	assert_eq!(chapter.chapter_number, Some(3.5));
	assert!(chapter.title.is_none());
	let bad: Title = serde_json::from_str(
		r#"{"title":" ","cover":"https://example.com/cover","url":"https://example.com/manga/key"}"#,
	)
	.unwrap();
	assert!(Manga::try_from(bad).is_err());
}

#[aidoku_test]
fn review_hero_metadata_and_sorted_tags() {
	let html = Html::parse(r#"<div class="carousel-slide"><a><img src="https://example.com/cover"></a><span class="uppercase">Manhwa</span><h2><a href="https://example.com/manga/key">Title</a></h2><div class="hidden"><span>Romance</span><span>Action</span></div><p>One &amp; two, [three].</p></div>"#).unwrap();
	let featured = home::parse_featured(&html, &[]);
	assert_eq!(
		featured[0].description.as_deref(),
		Some("One & two, [three].")
	);
	assert_eq!(featured[0].tags.as_ref().unwrap(), &["Action", "Romance"]);
	assert!(home::parse_featured(&html, &["Manhwa".into()]).is_empty());
}

#[aidoku_test]
fn review_novel_rows_use_real_chapters_and_dates() {
	let html = Html::parse(r#"<div id="novels-slider-container"><div class="group"><a href="https://example.com/manga/key"><img alt="Title" src="cover"></a><div><a href="https://example.com/read/key/ch12-123">Ch.&nbsp;12</a><span>7s ago</span></div></div></div>"#).unwrap();
	let entries = home::parse_novels(&html);
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].chapter.key, "key/ch12-123");
	assert_eq!(entries[0].chapter.chapter_number, Some(12.0));
	assert!(entries[0].chapter.date_uploaded.is_some());
	assert!(date("unknown").is_none());
}

#[aidoku_test]
fn review_novel_paragraphs_emphasis_and_credits() {
	assert_eq!(
		markdown::parse("<p>One <strong>bold</strong> line.</p><p><em>Another</em><br>line.</p>")
			.unwrap(),
		"One **bold** line.\n\n*Another*\n\nline."
	);
	assert!(markdown::parse("<div></div>").unwrap().is_empty());
	assert_eq!(credits("A, B & A, , C"), ["A", "B", "C"]);
	let html = Html::parse("<div class=manga-page-wrapper><a href='/tag/z'>Z</a><a href='/tag/a'>A</a><a href='/tag/z'>Z</a></div><a href='/tag/comment'>Comment</a>").unwrap();
	assert_eq!(tags(&html), ["A", "Z"]);
}

#[aidoku_test]
fn review_reader_errors_never_drop_missing_pages() {
	use crate::models::ChapterContent;
	use aidoku::{ContentRating, Page};
	for json in [
		r#"{"success":false,"message":"Unavailable"}"#,
		r#"{"success":true,"chapter_type":"media","images":["https://example.com/1",""]}"#,
		r#"{"success":true,"chapter_type":"media","images":[]}"#,
		r#"{"success":true,"chapter_type":"text","content":"<p></p>"}"#,
	] {
		let content: ChapterContent = serde_json::from_str(json).unwrap();
		assert!(Vec::<Page>::try_from(content).is_err());
	}
	for (tag, expected) in [
		("Hentai", ContentRating::NSFW),
		("Ecchi", ContentRating::Suggestive),
		("Action", ContentRating::Safe),
	] {
		assert_eq!(crate::helpers::rating(&[tag.into()]), expected);
	}
}
