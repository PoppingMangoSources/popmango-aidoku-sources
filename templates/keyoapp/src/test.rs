#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{HomeComponentValue, alloc::format, imports::html::Html};
use aidoku_test::aidoku_test;

struct Site;

impl Impl for Site {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: "https://example.com".into(),
			..Default::default()
		}
	}
}

#[aidoku_test]
fn review_series_links_keep_the_catalogue_key() {
	let source = Keyoapp::<Site>::new();
	for path in [
		"/series/title",
		"/series/title/",
		"/series/title/?utm_source=share#chapters",
	] {
		assert_eq!(
			source
				.handle_deep_link(format!("https://example.com{path}"))
				.unwrap(),
			Some(DeepLinkResult::Manga {
				key: "/series/title/".into()
			})
		);
	}
}

#[aidoku_test]
fn review_novel_emphasis_stays_inside_each_paragraph() {
	let html = Html::parse(
		"<div><p><strong>first<br>second</strong></p><p><em>third<br>fourth</em></p></div>",
	)
	.unwrap();
	let reader = html.select_first("div").unwrap();
	assert_eq!(
		helpers::novel_text(&reader).as_deref(),
		Some("**first**\n\n**second**\n\n*third*\n\n*fourth*")
	);
}

#[aidoku_test]
fn review_novel_prose_does_not_escape_ordinary_punctuation() {
	let text = "Hello, world. Wait: yes! ".repeat(500);
	let html = Html::parse(format!("<p>{text}</p>")).unwrap();
	let reader = html.select_first("html").unwrap();
	assert_eq!(helpers::novel_text(&reader).as_deref(), Some(text.trim()));
}

#[aidoku_test]
fn review_pasted_link_matches_absolute_catalogue_links() {
	let html = Html::parse(r#"<div id="searched_series_page"><button title="Title"><a title="Title" href="https://example.com/series/title/"><div style="background-image:url(https://example.com/cover.jpg)"></div></a></button></div>"#).unwrap();
	let result = Site
		.parse_search(
			&Site.params(),
			&html,
			Some("https://example.com/series/title".into()),
			Vec::new(),
		)
		.unwrap();
	assert_eq!(result.entries.len(), 1);
	assert_eq!(result.entries[0].key, "/series/title/");
}

#[aidoku_test]
fn review_unexpected_catalogue_html_fails() {
	let html = Html::parse("<h1>Unavailable</h1>").unwrap();
	assert!(
		Site.parse_search(&Site.params(), &html, None, Vec::new())
			.is_err()
	);
	let empty = Html::parse(r#"<div id="searched_series_page"></div>"#).unwrap();
	assert!(
		Site.parse_search(&Site.params(), &empty, None, Vec::new())
			.unwrap()
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn review_cards_require_a_nonempty_title() {
	let html = Html::parse(r#"<a href="/series/title/" title=" "></a>"#).unwrap();
	let link = html.select_first("a").unwrap();
	assert!(helpers::manga_from_link(&link, &link, "https://example.com").is_none());
	assert!(helpers::manga_from_card(&link, "https://example.com").is_none());
	let html = Html::parse(r#"<a href="/title" title=" "><h3>Title</h3></a>"#).unwrap();
	let link = html.select_first("a").unwrap();
	assert_eq!(
		helpers::manga_from_card(&link, "https://example.com")
			.unwrap()
			.title,
		"Title"
	);
	let html = Html::parse(r#"<div data-series-uid="1" data-series-slug="title"><h1> </h1></div>"#)
		.unwrap();
	assert!(helpers::manga_from_page(&html).is_none());
}

#[aidoku_test]
fn review_unavailable_chapters_are_skipped() {
	let html = Html::parse(r#"<div><a href="/chapter/paid" title="Chapter 2"><img alt="Coin"></a><a href="/chapter/future" title="Chapter 3"><span>Upcoming</span></a></div>"#).unwrap();
	for link in html.select("a").unwrap() {
		assert!(helpers::chapter_from_link(&link, "https://example.com").is_none());
	}
}

#[aidoku_test]
fn review_newly_added_stays_with_its_cover_grid() {
	for html in [
		r#"<section id="home-newly-added"><h2>Newly Added Series</h2><a class="group" href="/new" title="New"><img data-src="https://example.com/new.webp"></a></section>"#,
		r#"<section id="home-newly-added"><h2>Newly Added Series</h2></section><div id="series-grid"><a class="group" href="/new" title="New"><img data-src="https://example.com/new.webp"></a></div>"#,
	] {
		let html = Html::parse(html).unwrap();
		let home = home::parse_home_v2(&Site.params(), &html);
		assert_eq!(home.components.len(), 1);
		assert_eq!(
			home.components[0].title.as_deref(),
			Some("Newly Added Series")
		);
		let HomeComponentValue::Scroller { entries, .. } = &home.components[0].value else {
			panic!("expected covers");
		};
		assert_eq!(entries.len(), 1);
		assert_eq!(entries[0].title, "New");
		assert!(entries[0].subtitle.is_none());
	}
	let html = Html::parse(r#"<section id="home-newly-added"><h2>Newly Added Series</h2></section><section><h2>Other Section</h2></section><div id="series-grid"><a class="group" href="/unrelated" title="Unrelated"></a></div>"#).unwrap();
	assert!(
		home::parse_home_v2(&Site.params(), &html)
			.components
			.is_empty()
	);
}

#[aidoku_test]
fn review_novel_block_syntax_stays_literal_after_breaks() {
	let html =
		Html::parse("<div><p>Intro<br># Literal<br>1. Literal<br>---<br>  # Literal</p></div>")
			.unwrap();
	let reader = html.select_first("div").unwrap();
	assert_eq!(
		helpers::novel_text(&reader).as_deref(),
		Some("Intro\n\n\\# Literal\n\n1\\. Literal\n\n\\---\n\n\\# Literal")
	);
}
