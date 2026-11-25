#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{DeepLinkHandler, HomeComponentValue, alloc::vec, imports::html::Html};
use aidoku_test::aidoku_test;

fn flight_script(chunk: &str) -> String {
	let literal = serde_json::to_string(chunk).unwrap();
	format!("<script>self.__next_f.push([1,{literal}])</script>")
}

#[aidoku_test]
fn review_home_layout_order_and_components() {
	let flight = flight_script(
		"1:{\"banners\":[{\"series_slug\":\"hero-a\",\"title\":\"Hero A\",\"protagonist\":\"https://media.templetoons.com/p.webp\",\"description\":\"<p>A featured series.</p>\",\"total_views\":12000}],\"weekRes\":[{\"xSlug\":\"trend-w\",\"yName\":\"Trending Weekly\",\"thumbnail\":\"https://media.templetoons.com/tw.webp\",\"badge\":\"Manhwa\"}]}\n",
	);
	let html_str = format!(
		r#"<html><body>
		{flight}
		<section>
			<h2 id="latest-updates-title">Latest Updates</h2>
			<article data-sponsored="true"><a href="/comic/ad"><img src="/ad.webp"/></a><h3><a href="/comic/ad">Ad</a></h3></article>
			<article>
				<a href="/comic/orchid"><img src="https://media.templetoons.com/o.webp"/></a>
				<h3><a href="/comic/orchid">Until He Becomes An Orchid</a></h3>
				<ul>
					<li><a href="/comic/orchid/chapter-29"><span>Chapter 29</span><time datetime="2026-11-24T12:00:00Z">1d</time></a></li>
				</ul>
			</article>
		</section>
		<section>
			<h2>New Series</h2>
			<a href="/comic/new-one"><img src="https://media.templetoons.com/n.webp"/><h3>New One</h3></a>
		</section>
		</body></html>"#
	);
	let doc = Html::parse_with_url(&html_str, DOMAIN).unwrap();
	let layout = build_home_layout(&doc);

	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Featured",
			"Trending This Week",
			"Latest Updates",
			"New Series"
		]
	);
	assert!(matches!(
		layout.components[0].value,
		HomeComponentValue::BigScroller { .. }
	));
	assert!(matches!(
		layout.components[1].value,
		HomeComponentValue::Scroller { .. }
	));
	match &layout.components[2].value {
		HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries,
			..
		} => {
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].manga.key, "orchid");
			assert_eq!(entries[0].chapter.chapter_number, Some(29.0));
			assert!(entries[0].chapter.date_uploaded.is_some());
		}
		_ => panic!("expected Latest Updates MangaChapterList with page_size 5"),
	}
	assert!(matches!(
		layout.components[3].value,
		HomeComponentValue::Scroller { .. }
	));
}

#[aidoku_test]
fn review_catalog_details_chapters_and_reader_tokens() {
	// Renamed catalog fields
	let cat_flight = flight_script(
		"1:{\"items\":[{\"aaa\":\"evil-husband\",\"bbb\":\"Evil Husband\",\"thumbnail\":\"https://media.templetoons.com/e.webp\",\"badge\":\"Manhwa +18\",\"status\":\"Ongoing\",\"total_views\":5000,\"created_at\":\"2026-01-01T00:00:00Z\",\"update_chapter\":\"2026-11-20T00:00:00Z\"},{\"aaa\":\"switch-boyfriend\",\"bbb\":\"Switch Boyfriend\",\"thumbnail\":\"https://media.templetoons.com/s.webp\",\"badge\":\"Manhwa\",\"status\":\"Completed\",\"total_views\":2000,\"created_at\":\"2026-06-01T00:00:00Z\",\"update_chapter\":\"2026-11-10T00:00:00Z\"}]}\n",
	);
	let cat_doc =
		Html::parse_with_url(format!("<html><body>{cat_flight}</body></html>"), DOMAIN).unwrap();
	let items = parse_catalog(&cat_doc);
	assert_eq!(items.len(), 2);
	assert_eq!(items[0].slug, "evil-husband");
	assert_eq!(items[0].title, "Evil Husband");

	// Series details & chapters
	let detail_html = r#"<html><head><meta property="og:image" content="https://media.templetoons.com/c.webp"/></head><body>
		<h1 id="series-title">Evil Husband</h1>
		<div id="series-synopsis-text"><p>First paragraph.</p><p>Second paragraph.</p></div>
		<dl><div><dt>Author</dt><dd>Author One, Author Two</dd></div></dl>
		<ul aria-label="Series stats"><li>Ongoing</li></ul>
		<ul aria-label="Genres"><li>Romance</li><li>+18</li></ul>
		<ul id="chapter-list">
			<li><a href="/comic/evil-husband/chapter-52"><span>Chapter 52</span><span>Premium</span><time datetime="2026-11-24T10:00:00Z">1h</time></a></li>
			<li><a href="/comic/evil-husband/chapter-51"><span>Chapter 51</span><span>A Fresh Start</span><time datetime="2026-11-20T10:00:00Z">4d</time></a></li>
		</ul>
	</body></html>"#;
	let detail_doc = Html::parse_with_url(detail_html, DOMAIN).unwrap();
	let mut manga = Manga {
		key: "evil-husband".into(),
		title: String::new(),
		..Default::default()
	};
	parse_series_details(&detail_doc, &mut manga);
	assert_eq!(manga.title, "Evil Husband");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(
		manga.authors,
		Some(vec!["Author One".into(), "Author Two".into()])
	);

	let chapters = parse_series_chapters(&detail_doc, "evil-husband");
	assert_eq!(chapters.len(), 2);
	assert!(chapters[0].locked);
	assert_eq!(chapters[0].chapter_number, Some(52.0));
	assert!(!chapters[1].locked);
	assert_eq!(chapters[1].chapter_number, Some(51.0));
	assert_eq!(chapters[1].title.as_deref(), Some("A Fresh Start"));
	assert!(chapters[1].date_uploaded.is_some());

	// Reader token decoding & chunk seed recovery
	let page0 = "https://media.templetoons.com/file/p0.webp";
	let page1 = "https://media.templetoons.com/file/p1.webp";
	let tok0 = tokens::scramble(page0, 0, tokens::SEED);
	let tok1 = tokens::scramble(page1, 1, tokens::SEED);
	let reader_flight = flight_script(&format!(
		"2c:I[44083,[\"static/chunks/reader.js\"],\"default\"]\nc:[[\"$\",\"$L2c\",null,{{\"layoutHints\":[null,null],\"k\":[\"{tok0}\",\"{tok1}\"]}}]]\n"
	));
	let reader_doc =
		Html::parse_with_url(format!("<html><body>{reader_flight}</body></html>"), DOMAIN).unwrap();
	let reader = extract_reader_tokens(&reader_doc).unwrap();
	assert_eq!(
		reader.script,
		Some((44083, "static/chunks/reader.js".into()))
	);
	assert_eq!(
		decode_reader_tokens(&reader.tokens, tokens::SEED).unwrap(),
		[page0, page1]
	);

	let chunk = "44083:function(e,t){const s=\"abc123456789\";new TextDecoder();const a=795580989,b=2048675417;}";
	assert_eq!(
		tokens::seed_from_chunk(chunk, 44083).as_deref(),
		Some("abc123456789")
	);

	// Deep links & Turnstile detection
	let source = TempleScan::new();
	assert!(matches!(
		source
			.handle_deep_link(format!("{DOMAIN}/comic/evil-husband/chapter-51"))
			.unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == "evil-husband" && key == "chapter-51"
	));
	let cf_doc = Html::parse_with_url(
		"<html><head><title>Just a moment...</title></head></html>",
		DOMAIN,
	)
	.unwrap();
	assert!(is_challenge_page(&cf_doc));
}
