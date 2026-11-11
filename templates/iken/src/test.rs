use super::*;
use aidoku::println;
use aidoku::{
	HomeComponentValue, LinkValue,
	alloc::{format, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_novel_emphasis_and_paragraph_boundaries() {
	assert_eq!(
		markdown::parse(
			"<p><b><i>one</i></b><b><i>two</i></b></p><p><strong>three<br>four</strong></p>"
		)
		.unwrap(),
		"***onetwo***\n\n**three**\n\n**four**"
	);
	let prose = "Hello, world. Wait: yes! ".repeat(500);
	assert_eq!(markdown::parse(&prose).unwrap(), prose.trim());
	assert!(markdown::parse("<script>hidden</script><p> </p>").is_err());
}

#[aidoku_test]
fn review_independent_locks_and_numeric_strings() {
	for flag in [
		"isLocked",
		"isTimeLocked",
		"isPermanentlyLocked",
		"isLockedByCoins",
		"isShortLinkLocked",
	] {
		let chapter: models::IkenChapter = serde_json::from_str(
			&format!(
				r#"{{"id":1,"slug":"chapter-1","number":"1.5","isLocked":false,"{flag}":true}}"#
			)
			.replace("\"isLocked\":false,\"isLocked\":true", "\"isLocked\":true"),
		)
		.unwrap();
		assert!(chapter.locked());
		assert_eq!(
			chapter
				.into_chapter("https://example.com", Some("title"))
				.unwrap()
				.chapter_number,
			Some(1.5)
		);
	}
	let chapter: models::IkenChapter = serde_json::from_str(
		r#"{"id":1,"slug":"chapter-1","number":1,"price":100,"isAccessible":true}"#,
	)
	.unwrap();
	assert!(!chapter.locked());
}

#[aidoku_test]
fn review_identity_compact_cards_and_blank_metadata() {
	let post: models::Post = serde_json::from_str(r#"{"id":7,"slug":"title","postTitle":" Title ","author":"  ","artist":" ","postContent":"<p> </p>","is18Plus":true,"genres":[{"name":"Z"},{"name":"A"}]}"#).unwrap();
	let basic = Manga::try_from(&post).unwrap();
	assert_eq!(basic.key, "7");
	assert_eq!(basic.title, "Title");
	assert!(basic.tags.is_none() && basic.authors.is_none() && basic.description.is_none());
	let detail = post.into_details("https://example.com").unwrap();
	assert!(detail.description.is_none() && detail.authors.is_none() && detail.artists.is_none());
	assert_eq!(detail.content_rating, aidoku::ContentRating::NSFW);
	assert_eq!(detail.tags, Some(vec!["A".into(), "Z".into()]));
	for (id, slug, title) in [(0, "title", "Title"), (1, " ", "Title"), (1, "title", " ")] {
		let post: models::Post = serde_json::from_str(&format!(
			r#"{{"id":{id},"slug":"{slug}","postTitle":"{title}"}}"#
		))
		.unwrap();
		assert!(Manga::try_from(&post).is_err());
	}
}

#[aidoku_test]
fn review_home_order_compact_cards_and_section_boundaries() {
	let html = Html::parse(r#"<main><section><div><h2>Latest Releases</h2></div><div><a href='/series/one' title=' One '><img src='https://example.com/one.jpg'></a><a href='/series/one' title='One'><img src='same'></a><a href='/series/blank' title=' '><img src='blank'></a></div></section><section><h2>Latest Updates</h2></section><section><h2>Most Popular</h2><a href='/series/two' title='Two'><img src='two'></a></section></main>"#).unwrap();
	let params = Params {
		base_url: "https://example.com".into(),
		..Default::default()
	};
	let layout = home::parse(&html, &params).unwrap();
	assert_eq!(layout.components.len(), 2);
	assert_eq!(
		layout.components[0].title.as_deref(),
		Some("Latest Releases")
	);
	assert_eq!(layout.components[1].title.as_deref(), Some("Most Popular"));
	let HomeComponentValue::Scroller { entries, .. } = &layout.components[0].value else {
		panic!("Expected scroller");
	};
	assert_eq!(entries.len(), 1);
	assert!(entries[0].subtitle.is_none());
	let Some(LinkValue::Manga(manga)) = &entries[0].value else {
		panic!("Expected manga link");
	};
	assert_eq!(manga.key, "/series/one");
	assert!(manga.description.is_none());
	assert!(matches!(
		layout.components[1].value,
		HomeComponentValue::MangaList {
			page_size: Some(5),
			..
		}
	));
}

#[aidoku_test]
#[ignore]
fn review_http_errors_and_key_only_reading() {
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("Run scripts/test-http.py");
	};
	for status in [403, 404, 429, 503] {
		let url = format!("{base}/status/{status}");
		assert!(
			helpers::check_response(Request::get(&url).unwrap().send().unwrap(), &url).is_err()
		);
	}
	let params = Params {
		base_url: base.into(),
		..Default::default()
	};
	struct Site;
	impl Impl for Site {
		fn new() -> Self {
			Self
		}
		fn params(&self) -> Params {
			Params::default()
		}
	}
	assert_eq!(
		Site.get_page_list(
			&params,
			Manga {
				key: "7".into(),
				..Default::default()
			},
			Chapter {
				key: "11".into(),
				..Default::default()
			}
		)
		.unwrap()[0]
			.content,
		aidoku::PageContent::text("**Text**")
	);
	assert!(
		Site.get_page_list(
			&params,
			Manga::default(),
			Chapter {
				key: "12".into(),
				..Default::default()
			}
		)
		.is_err()
	);
	assert!(
		Site.get_page_list(
			&params,
			Manga::default(),
			Chapter {
				key: "13".into(),
				..Default::default()
			}
		)
		.is_err()
	);
	assert_eq!(helpers::chapters(&params, "7").unwrap().len(), 3);
	for id in ["998", "999"] {
		assert!(helpers::chapters(&params, id).is_err());
	}
}

#[aidoku_test]
#[ignore]
fn review_http_captured_homes() {
	if option_env!("AIDOKU_IKEN_EVIDENCE_DIR").is_none() {
		return;
	}
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("Run scripts/test-http.py");
	};
	for (site, host) in [
		("hive", "hivetoons.org"),
		("nyx", "nyxscans.com"),
		("vortex", "vortexscans.org"),
		("magus", "magustoon.org"),
		("eternal", "eternalmangas.org"),
	] {
		let html = Request::get(format!("{base}/captured/home/{site}"))
			.unwrap()
			.html()
			.unwrap();
		let mut params = Params {
			base_url: format!("https://{host}").into(),
			..Default::default()
		};
		if site == "eternal" {
			params.home_slider_selector = ".em-hero .carousel-item".into();
			params.home_section_titles = &[
				"Popular Hoy!",
				"Últimas Actualizaciones",
				"Tendencias",
				"Novelas",
			];
		}
		let layout = home::parse(&html, &params).unwrap();
		println!(
			"Captured {site}: {:?}",
			layout
				.components
				.iter()
				.map(|c| c.title.as_deref())
				.collect::<Vec<_>>()
		);
		assert_eq!(
			layout.components.len(),
			if site == "hive" {
				2
			} else if site == "magus" {
				5
			} else {
				4
			}
		);
	}
}
