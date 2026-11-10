#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::imports::html::Html;
use aidoku::{AidokuError, prelude::*};
use aidoku_test::aidoku_test;

struct Site;

impl Impl for Site {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params::default()
	}
}

#[aidoku_test]
fn review_novel_emphasis_keeps_paragraphs_and_breaks() {
	for (body, expected) in [
		(
			"<p><strong><b>Bold</b></strong> <em><i>Italic</i></em></p>",
			"**Bold** *Italic*",
		),
		(
			"<p>Plain <strong>bold</strong> and <em>italic</em>.</p>",
			"Plain **bold** and *italic*.",
		),
		(
			"<div><p><b>First<br>Second</b></p><div><i>Third<br/>Fourth</i></div></div>",
			"**First**\n\n**Second**\n\n*Third*\n\n*Fourth*",
		),
		(
			"<p><b><i>Both</i></b></p><p><strong>One</strong><strong>Two</strong></p>",
			"***Both***\n\n**OneTwo**",
		),
		(
			"<p>Before<strong> bold </strong>after <em> </em>end</p>",
			"Before **bold** after end",
		),
		(
			"<div>First<p>Second</p>Third</div>",
			"First\n\nSecond\n\nThird",
		),
	] {
		let html = Html::parse(format!("<div id=readerarea>{body}</div>")).unwrap();
		assert_eq!(helpers::novel_text(&html), expected);
	}
}

#[aidoku_test]
fn review_novel_prose_escapes_syntax_without_expanding_punctuation() {
	let prose = "Hello, world. Wait: yes! ".repeat(500);
	let html = Html::parse(format!("<div id=readerarea><p>{prose}</p><p>1. *Literal* &amp; now</p><script>hidden</script><style>hidden</style></div>")).unwrap();
	assert_eq!(
		helpers::novel_text(&html),
		format!("{}\n\n1\\. \\*Literal\\* \\& now", prose.trim())
	);
}

#[aidoku_test]
fn review_novel_empty_content_stays_empty() {
	for body in ["", "<p> </p>", "<p><b></b><i> </i></p>"] {
		let html = Html::parse(format!("<div id=readerarea>{body}</div>")).unwrap();
		assert!(helpers::novel_text(&html).is_empty());
	}
}

fn assert_http<T>(result: Result<T>, status: i32) {
	let Err(AidokuError::Message(message)) = result else {
		panic!("expected HTTP {status}");
	};
	assert_eq!(message, format!("HTTP {status}"));
}

// The loopback server is started by scripts/test-http.py.
#[aidoku_test]
#[ignore]
fn review_http_status_and_diagnostics() {
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("run scripts/test-http.py");
	};
	let site = Site::new();
	for status in [403, 404, 429, 503] {
		let mut params = site.params();
		params.base_url = format!("{base}/status/{status}").into();
		assert_http(
			site.get_search_manga_list(&params, None, 1, Vec::new()),
			status,
		);
		assert_http(
			site.get_manga_update(
				&params,
				Manga {
					key: "/title/".into(),
					..Default::default()
				},
				true,
				true,
			),
			status,
		);
		assert_http(
			site.get_page_list(
				&params,
				Manga::default(),
				Chapter {
					key: "/chapter/1/".into(),
					..Default::default()
				},
			),
			status,
		);
		assert_http(site.get_home(&params), status);
		assert_http(child_theme::get_home(&params), status);
		let link = child_theme::handle_deep_link(&params, format!("{}/title/", params.base_url));
		if status == 404 {
			assert_eq!(link.unwrap(), None);
		} else {
			assert_http(link, status);
		}
	}
	for status in [200, 201, 204, 299] {
		assert_eq!(
			helpers::fetch(&format!("{base}/status/{status}"))
				.unwrap()
				.status_code(),
			status
		);
	}
}

#[aidoku_test]
#[ignore]
fn review_http_viewer_override_preserves_default_type_mapping() {
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("run scripts/test-http.py");
	};
	for (kind, expected) in [
		("Manga", Viewer::RightToLeft),
		("Manhwa", Viewer::Webtoon),
		("Comic", Viewer::LeftToRight),
	] {
		for override_viewer in [None, Some(Viewer::Webtoon)] {
			let params = Params {
				base_url: base.into(),
				series_type_selector: ".type".into(),
				viewer_override: override_viewer,
				..Default::default()
			};
			let manga = Site
				.get_manga_update(
					&params,
					Manga {
						key: format!("/metadata/{kind}"),
						..Default::default()
					},
					true,
					false,
				)
				.unwrap();
			assert_eq!(manga.viewer, override_viewer.unwrap_or(expected));
		}
	}
}

#[aidoku_test]
#[ignore]
fn review_http_novel_reader_uses_only_chapter_key() {
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("run scripts/test-http.py");
	};
	let params = Params {
		base_url: base.into(),
		..Default::default()
	};
	let pages = Site
		.get_page_list(
			&params,
			Manga::default(),
			Chapter {
				key: "/novel".into(),
				..Default::default()
			},
		)
		.unwrap();
	let [
		Page {
			content: aidoku::PageContent::Text(text),
			..
		},
	] = pages.as_slice()
	else {
		panic!("Expected one text page");
	};
	assert_eq!(text, "**First**\n\n**Second**\n\n*Third*");
}
