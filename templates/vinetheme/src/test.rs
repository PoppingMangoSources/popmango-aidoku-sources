use crate::{
	helpers, markdown,
	models::{ChapterPages, Series},
	rsc,
};
use aidoku::{
	Manga,
	alloc::{String, format, vec},
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_relative_dates_cover_the_site_suffixes() {
	let now = aidoku::imports::std::current_date();
	for (label, seconds) in [
		("9m", 9 * 60),
		("6h", 6 * 3600),
		("29d", 29 * 86400),
		("5mo", 5 * 2_592_000),
	] {
		let date = crate::home::relative_date(label).unwrap();
		assert!((now - date - seconds).abs() < 5, "{label}");
	}
	assert!(crate::home::relative_date("soon").is_none());
}

#[aidoku_test]
fn review_credit_names_split_slashes() {
	assert_eq!(
		crate::helpers::credit_names(Some("Writer / Artist, Writer".into())),
		Some(vec!["Writer".into(), "Artist".into()])
	);
}

#[aidoku_test]
fn review_reader_skips_chapter_metadata() {
	let body = "1:[\"$\",{\"chapter\":{\"id\":\"entry\",\"number\":1}}]\n2:[\"$\",{\"chapter\":{\"content\":\"\",\"pages\":[{\"imageUrl\":\"/page.webp\"}]}}]";
	let reader: ChapterPages = rsc::extract(body, &["pages", "content"]).unwrap();
	assert_eq!(reader.pages.len(), 1);
	assert_eq!(reader.pages[0].image_url.as_deref(), Some("/page.webp"));
}

#[aidoku_test]
fn review_text_references_preserve_multibyte_content() {
	let text = "<p>First 界</p>\n<p><em>Second</em></p>";
	let body = format!(
		"1:{{\"content\":\"$a\",\"pages\":[]}}\na:T{:x},{text}\n2:null",
		text.len()
	);
	assert_eq!(rsc::text(&body, "$a").unwrap(), text);
	assert!(rsc::text("a:T20,short", "$a").is_err());
}

#[aidoku_test]
fn review_key_only_routes_keep_comics_and_novels_separate() {
	assert_eq!(helpers::series_path("title"), "comic/title");
	assert_eq!(helpers::series_path("novel/title"), "novel/title");
	assert_eq!(
		helpers::link_keys("https://example.com/series/novel/title/chapter/2.5/?ref=home").unwrap(),
		("novel/title".into(), Some("2.5".into()))
	);
	assert!(helpers::link_keys("https://example.com/series/comic//chapter/1").is_none());
}

#[aidoku_test]
fn review_novel_paragraphs_and_nested_emphasis() {
	assert_eq!(
		markdown::parse("<p><strong>First<br><em>Second</em></strong></p><p>Third.</p>").unwrap(),
		"**First**\n\n***Second***\n\nThird."
	);
	assert!(markdown::parse("<p> </p>").unwrap().is_empty());
}

#[aidoku_test]
fn review_comics_open_as_webtoons_and_novels_do_not() {
	use aidoku::Viewer;
	for (kind, viewer) in [
		("MANGA", Viewer::Webtoon),
		("MANHWA", Viewer::Webtoon),
		("", Viewer::Webtoon),
		("WEB_NOVEL", Viewer::Unknown),
	] {
		let series = serde_json::from_value::<Series>(serde_json::json!({
			"slug": "title",
			"title": "Title",
			"type": kind,
		}))
		.unwrap();
		assert_eq!(Manga::try_from(series).unwrap().viewer, viewer, "{kind}");
	}
}

#[aidoku_test]
fn review_empty_html_synopsis_keeps_valid_series() {
	for description in ["", " ", "<p></p>", "<p>&nbsp;</p>"] {
		let series = serde_json::from_value::<Series>(serde_json::json!({
			"slug": "title",
			"title": "Title",
			"description": description,
		}))
		.unwrap();
		let manga = Manga::try_from(series).unwrap();
		assert_eq!(manga.key, "title");
		assert_eq!(manga.title, "Title");
		assert!(manga.description.is_none());
	}
}

#[aidoku_test]
fn review_full_html_scripts_decode_before_extraction() {
	let flight = "1:{\"pages\":[],\"content\":\"<p>Text</p>\"}\n";
	let json = serde_json::to_string(&aidoku::alloc::vec![
		serde_json::json!(1),
		serde_json::json!(flight)
	])
	.unwrap();
	let html = format!("<html><body><script>self.__next_f.push({json})</script></body></html>");
	assert_eq!(rsc::decode(&html).unwrap(), flight);
}

#[aidoku_test]
fn review_text_rows_do_not_hide_following_payloads() {
	let body = "1:T3,界2:{\"pages\":[],\"content\":\"Text\"}\n";
	let chapter: ChapterPages = rsc::extract(body, &["pages", "content"]).unwrap();
	assert_eq!(chapter.content.as_deref(), Some("Text"));
}

#[aidoku_test]
fn review_flight_hints_do_not_hide_reader_objects() {
	let body = ":HL[\"/style.css\",\"style\"]\n1:{\"pages\":[],\"content\":\"Text\"}\n";
	let chapter: ChapterPages = rsc::extract(body, &["pages", "content"]).unwrap();
	assert_eq!(chapter.content.as_deref(), Some("Text"));
}

#[aidoku_test]
fn review_featured_stack_keeps_valid_links_without_array() {
	let flight = serde_json::json!([
		1,
		"1:{\"all\":[{\"slug\":\"pick\",\"title\":\"Pick\",\"coverImage\":\"/cover.webp\"}]}"
	]);
	let body = format!(
		"<html><section><h2>Editor's Pick</h2><div data-stack-frame><a href='/series/comic/pick?ref=homepage_featured'>Read Now</a></div></section><script>self.__next_f.push({flight})</script></html>"
	);
	let home = crate::home::parse(
		&body,
		&crate::Params {
			base_url: "https://example.com".into(),
		},
	)
	.unwrap();
	assert!(
		matches!(&home.components[0].value, aidoku::HomeComponentValue::BigScroller { entries, .. } if entries.len() == 1 && entries[0].key == "pick")
	);
}

#[aidoku_test]
fn review_home_enriches_partial_hero_and_adjacent_text_rows() {
	let text = "<p>First 界</p>\n<p><b>Second</b></p>";
	let flight = format!(
		"1:{{\"initialSlides\":[{{\"slug\":\"title\",\"title\":\"Title\",\"coverImage\":\"/cover.webp\"}}],\"all\":[{{\"slug\":\"title\",\"title\":\"Title\",\"coverImage\":\"/cover.webp\",\"description\":\"$99\",\"genres\":[\"Drama\"]}}]}}\n2:T3,界99:T{:x},{text}",
		text.len()
	);
	assert_eq!(rsc::text(&flight, "$99").unwrap(), text);
	let body = format!(
		"<html><body><script>self.__next_f.push({})</script></body></html>",
		serde_json::json!([1, flight])
	);
	let home = crate::home::parse(
		&body,
		&crate::Params {
			base_url: "https://example.com".into(),
		},
	)
	.unwrap();
	assert!(
		matches!(&home.components[0].value, aidoku::HomeComponentValue::BigScroller { entries, .. } if entries[0].description.as_deref() == Some("First 界\nSecond") && entries[0].tags.as_ref().unwrap()[0] == "Drama")
	);
}

#[aidoku_test]
fn review_vine_tag_labels_use_readable_case() {
	let series: Series = serde_json::from_value(serde_json::json!({
		"slug": "title",
		"title": "Title",
		"genres": ["drama", {"name": "romance", "slug": "romance"}, {"name": "", "slug": "slice-of-life"}],
		"tags": ["Drama", "josei", {"name": "", "slug": "historical"}, "smut"]
	})).unwrap();
	let manga = Manga::try_from(series).unwrap();
	assert_eq!(
		manga.tags.unwrap(),
		vec![
			String::from("Drama"),
			String::from("Historical"),
			String::from("Josei"),
			String::from("Romance"),
			String::from("Slice Of Life"),
			String::from("Smut"),
		]
	);
	assert_eq!(manga.content_rating, aidoku::ContentRating::NSFW);
}
