use super::{
	TempleScan,
	helpers::{
		BASE_URL, decode_reader_tokens, extract_reader_tokens, is_challenge_page, page_catalog,
		parse_catalog, parse_series_chapters, parse_series_details, sort_catalog,
	},
	home::layout,
	tokens,
};
use aidoku::{
	DeepLinkHandler, DeepLinkResult, Home, HomeComponentValue, Manga, MangaStatus, Source,
	alloc::{String, Vec, format, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

// Featured description uses a `$29` Flight text reference whose HTML carries an entity.
const HOME_HTML: &str = r#"<!doctype html>
<html>
<head><title>Temple Scan</title></head>
<body>
<script>self.__next_f.push([1,"29:T1a,<p>A hero&#x27;s rise.</p>1:[\"$\",\"$L1\",null,{\"featured\":[{\"a\":\"solo-hunter\",\"b\":\"Solo Hunter\",\"protagonist\":\"/uploads/p1.webp\",\"description\":\"$29\",\"badge\":\"NEW\"}],\"weekRes\":[{\"x\":\"week-king\",\"y\":\"Week King\",\"thumbnail\":\"/uploads/w1.webp\",\"badge\":\"18+\"}]}]\n"])</script>
<section aria-labelledby="trending-title">
	<ol><li><a href="/comic/week-king"><img src="/uploads/w1.webp"><h3>Week King</h3></a></li></ol>
</section>
<section>
	<h2 id="latest-updates-title">Latest Updates</h2>
	<article>
		<a href="/comic/latest-one"><img src="/uploads/l1.webp"></a>
		<h3><a href="/comic/latest-one">Latest One</a></h3>
		<ul>
			<li><a href="/comic/latest-one/chapter-42"><span class="sr-only">Premium</span><span>Chapter 42</span><time datetime="2026-11-24T10:00:00.000Z">1h ago</time></a></li>
			<li><a href="/comic/latest-one/chapter-41"><span>Chapter 41</span><time datetime="2026-11-23T10:00:00.000Z">1d ago</time></a></li>
		</ul>
	</article>
	<article>
		<a href="/comic/locked-only"><img src="/uploads/l2.webp"></a>
		<h3><a href="/comic/locked-only">Locked Only</a></h3>
		<ul>
			<li><a href="/comic/locked-only/chapter-9"><span class="sr-only">Premium</span><span>Chapter 9</span><time datetime="2026-11-24T11:00:00.000Z">30m ago</time></a></li>
		</ul>
	</article>
</section>
<section>
	<h2>New Series</h2>
	<a href="/comic/brand-new"><img src="/uploads/n1.webp"><h3>Brand New</h3></a>
</section>
</body>
</html>"#;

#[aidoku_test]
fn review_home_layout_flight_refs_and_show_locked() {
	let doc = Html::parse_with_url(HOME_HTML, BASE_URL).expect("parse home");
	let built = layout(&doc, false);
	let titles: Vec<&str> = built
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		vec![
			"Featured",
			"Trending This Week",
			"Latest Updates",
			"New Series"
		]
	);

	match &built.components[0].value {
		HomeComponentValue::BigScroller { entries, .. } => {
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].key, "solo-hunter");
			assert_eq!(entries[0].description.as_deref(), Some("A hero's rise."));
		}
		_ => panic!("Featured must be BigScroller"),
	}
	match &built.components[1].value {
		HomeComponentValue::Scroller { entries, .. } => {
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].title, "Week King");
			assert!(entries[0].subtitle.is_none());
		}
		_ => panic!("Trending This Week must be Scroller"),
	}
	match &built.components[2].value {
		HomeComponentValue::MangaChapterList {
			page_size, entries, ..
		} => {
			assert_eq!(*page_size, Some(5));
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].manga.key, "latest-one");
			assert_eq!(entries[0].chapter.chapter_number, Some(41.0));
			assert!(entries[0].chapter.title.is_none());
			assert!(entries[0].chapter.url.is_none());
		}
		_ => panic!("Latest Updates must be MangaChapterList"),
	}
	match &built.components[3].value {
		HomeComponentValue::Scroller { entries, .. } => {
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].title, "Brand New");
		}
		_ => panic!("New Series must be Scroller"),
	}

	let layout_with_locked = layout(&doc, true);
	if let HomeComponentValue::MangaChapterList { entries, .. } =
		&layout_with_locked.components[2].value
	{
		assert_eq!(entries.len(), 2);
		assert!(entries[1].chapter.locked);
	} else {
		panic!("Latest Updates must be MangaChapterList");
	}
}

#[aidoku_test]
fn review_flight_text_rows_sorts_locked_chapters_and_reader_tokens() {
	let catalog_html = r#"<html><body><script>self.__next_f.push([1,"a:T5,hello1:[\"$\",\"$L2\",null,{\"initialPage\":1,\"list\":[{\"z1\":\"alpha-one\",\"z2\":\"Alpha One\",\"thumbnail\":\"/a.webp\",\"badge\":\"Safe\",\"status\":\"Ongoing\",\"created_at\":\"2026-01-01T00:00:00.000Z\",\"update_chapter\":\"2026-11-24T00:00:00.000Z\",\"total_views\":500,\"alternative_names\":\"Alt A\"},{\"z1\":\"beta-two\",\"z2\":\"Beta Two\",\"thumbnail\":\"/b.webp\",\"badge\":\"18+\",\"status\":\"Completed\",\"created_at\":\"2026-10-01T00:00:00.000Z\",\"update_chapter\":\"2026-11-20T00:00:00.000Z\",\"total_views\":1000,\"alternative_names\":\"\"}]}]\n"])</script></body></html>"#;
	let cat_doc = Html::parse_with_url(catalog_html, BASE_URL).expect("parse catalog");
	let mut items = parse_catalog(&cat_doc);
	assert_eq!(items.len(), 2);

	// 2 = Title, 3 = Trending (views). The runner drops a format's quoted 'T', so the date sorts
	// are checked in the app.
	sort_catalog(&mut items, 3, false);
	assert_eq!(items[0].slug, "beta-two");
	sort_catalog(&mut items, 2, true);
	let page = page_catalog(items, 1);
	assert_eq!(page.entries[0].key, "alpha-one");
	assert_eq!(page.entries[1].key, "beta-two");

	let challenge_html = r#"<html><head><title>Temple Scan</title><script src="/_next/static/chunks/app/challenge/page-61f1e8.js"></script></head><body></body></html>"#;
	let challenge_doc = Html::parse_with_url(challenge_html, BASE_URL).expect("parse challenge");
	assert!(is_challenge_page(&challenge_doc));
	let titled_html = r#"<html><head><title>The Verify Challenge - Temple Scan</title></head><body></body></html>"#;
	let titled_doc = Html::parse_with_url(titled_html, BASE_URL).expect("parse titled page");
	assert!(!is_challenge_page(&titled_doc));

	let detail_html = r#"<html><head><meta property="og:image" content="https://media.templetoons.com/c.webp"></head><body>
		<h1 id="series-title">Alpha One</h1>
		<div id="series-synopsis-text"><p>First para.</p><p>Second para.</p></div>
		<dl><div><dt>Author</dt><dd>Jane Doe</dd></div></dl>
		<ul aria-label="Series stats"><li>Ongoing</li></ul>
		<ul aria-label="Genres"><li>Fantasy</li><li>Action</li><li>Action</li></ul>
		<ul id="chapter-list">
			<li><a href="/comic/alpha-one/ch-3"><span class="sr-only">Premium</span><span>Chapter 3</span><time datetime="2026-11-25T12:00:00.000Z">1h</time></a></li>
			<li><a href="/comic/alpha-one/ch-2"><span>Chapter 2</span><span>Chapter 2</span><time datetime="2026-11-24T12:00:00.000Z">2h</time></a></li>
			<li><a href="/comic/alpha-one/ch-1"><span>Ch. 1</span><span>Prologue</span><time datetime="2026-11-20T12:00:00.000Z">4d</time></a></li>
		</ul>
	</body></html>"#;
	let detail_doc = Html::parse_with_url(detail_html, BASE_URL).expect("parse detail");
	let mut manga = Manga {
		key: "alpha-one".into(),
		..Default::default()
	};
	parse_series_details(&detail_doc, &mut manga).expect("details");
	assert_eq!(manga.title, "Alpha One");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(
		manga.tags,
		Some(vec![String::from("Action"), String::from("Fantasy")])
	);
	// Credits keep the site's order and drop repeats; a page with no title is an error.
	let credits = Html::parse_with_url(
		r#"<html><body><h1 id="series-title">Beta</h1><dl><div><dt>Author</dt><dd>Palgarak / Seokho, Palgarak | Kingdom Company</dd></div></dl></body></html>"#,
		BASE_URL,
	)
	.expect("parse credits");
	let mut beta = Manga::default();
	parse_series_details(&credits, &mut beta).expect("credits");
	assert_eq!(
		beta.authors,
		Some(vec![
			String::from("Palgarak"),
			String::from("Seokho"),
			String::from("Kingdom Company"),
		])
	);
	let untitled = Html::parse_with_url("<html><body></body></html>", BASE_URL).expect("parse");
	assert!(parse_series_details(&untitled, &mut Manga::default()).is_err());

	let free_chapters = parse_series_chapters(&detail_doc, "alpha-one", false);
	assert_eq!(free_chapters.len(), 2);
	assert_eq!(free_chapters[0].chapter_number, Some(2.0));
	assert!(free_chapters[0].title.is_none());
	assert_eq!(free_chapters[1].chapter_number, Some(1.0));
	assert_eq!(free_chapters[1].title.as_deref(), Some("Prologue"));

	let all_chapters = parse_series_chapters(&detail_doc, "alpha-one", true);
	assert_eq!(all_chapters.len(), 3);
	assert!(all_chapters[0].locked);

	let url0 = "https://media.templetoons.com/file/terms54/uploads/series/s/c/01.jpg";
	let url1 = "https://media.templetoons.com/file/terms54/uploads/series/s/c/02.jpg";
	// The two urls as the site's reader encodes them at positions 0 and 1.
	let t0 = "spfvfINeTrkSh2H2W9rQJ8ntZ0At1dEaK47f7qJxaZWDgGFHwt8mu7UQsL9h-vxAvwlNmylq16a0NEi5n6PdgrhCFJc";
	let t1 = "jowy6N22UpdUNT4M7xFHrRKmt1ppePuv2dLbDh38LjRksvzKXjES9zoutCHy8MwGpOPdSNO89ht2pOY5RI0nM6YrCr8";
	let reader_html = format!(
		r#"<html><body><script>self.__next_f.push([1,"5:I[44083,[\"static/chunks/reader.js\"],\"default\"]\n6:[\"$\",\"$L5\",null,{{\"layoutHints\":[1,1],\"p\":[\"{t0}\",\"{t1}\"]}}]\n"])</script></body></html>"#
	);
	let reader_doc = Html::parse_with_url(&reader_html, BASE_URL).expect("parse reader");
	let reader = extract_reader_tokens(&reader_doc).expect("reader tokens");
	let decoded = decode_reader_tokens(&reader.tokens, tokens::SEED).expect("decoded urls");
	assert_eq!(decoded, vec![String::from(url0), String::from(url1)]);

	let source = TempleScan::new();
	assert_eq!(
		source
			.handle_deep_link(format!("{BASE_URL}/comic/alpha-one/ch-2"))
			.expect("deep link"),
		Some(DeepLinkResult::Chapter {
			manga_key: "alpha-one".into(),
			key: "ch-2".into(),
		})
	);
}

// Without the browser headers every page request lands on the site's check page.
#[aidoku_test]
fn live_pages_pass_the_site_check() {
	let source = TempleScan::new();
	let browse = source
		.get_search_manga_list(None, 1, Vec::new())
		.expect("browse");
	assert!(!browse.entries.is_empty());

	let manga = source
		.get_manga_update(
			Manga {
				key: "the-lawful-wife".into(),
				..Default::default()
			},
			true,
			true,
		)
		.expect("details");
	assert_eq!(manga.title, "The Lawful Wife");
	let first = manga
		.chapters
		.clone()
		.expect("chapters")
		.into_iter()
		.find(|chapter| chapter.key == "chapter-1")
		.expect("chapter 1");
	assert!(
		!source
			.get_page_list(manga, first)
			.expect("pages")
			.is_empty()
	);

	assert!(!source.get_home().expect("home").components.is_empty());

	// A pasted link answers with a compact card; a dead one, or text that only mentions a path,
	// is searched as text.
	let pasted = source
		.get_search_manga_list(
			Some(format!("{BASE_URL}/comic/the-lawful-wife")),
			1,
			Vec::new(),
		)
		.expect("pasted link");
	assert_eq!(pasted.entries.len(), 1);
	assert_eq!(pasted.entries[0].title, "The Lawful Wife");
	assert!(pasted.entries[0].description.is_none() && pasted.entries[0].tags.is_none());
	for query in [
		format!("{BASE_URL}/comic/no-such-series-here"),
		String::from("see /comic/the-lawful-wife"),
	] {
		assert!(
			source
				.get_search_manga_list(Some(query), 1, Vec::new())
				.expect("text search")
				.entries
				.is_empty()
		);
	}
}
