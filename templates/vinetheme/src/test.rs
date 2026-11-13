use crate::{helpers, markdown, models::ChapterPages, rsc};
use aidoku::alloc::format;
use aidoku_test::aidoku_test;

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
	assert!(markdown::parse("<p> </p>").is_err());
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
