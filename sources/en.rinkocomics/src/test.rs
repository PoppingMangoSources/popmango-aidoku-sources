#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	DeepLinkHandler, DynamicFilters, Home, HomeComponentValue, ListingProvider, Source,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use base64::prelude::*;

#[aidoku_test]
fn review_home_layout_order_and_component_shapes() {
	let home_html = r#"
		<div data-cw-slider>
			<div data-slide>
				<a class="btn-primary" href="https://rinkocomics.com/comic/hero-a/">Read</a>
				<h2>Hero A</h2>
				<img src="https://rinkocomics.com/a.webp" />
				<p>A summary from the site.</p>
				<div class="flex-wrap"><span>Fantasy · Romance</span></div>
			</div>
		</div>
		<section aria-labelledby="cw-week-m">
			<a href="https://rinkocomics.com/comic/week-a/">
				<img src="https://rinkocomics.com/w.webp" />
				<span class="line-clamp-2">Week A</span>
			</a>
		</section>
		<section id="cw-latest">
			<article>
				<a class="line-clamp-2" href="https://rinkocomics.com/comic/lat-c/">Latest Comic</a>
				<img src="https://rinkocomics.com/lc.webp" />
				<a class="chapter-item" href="https://rinkocomics.com/chapter/lat-c-ch-5/">
					<span>Chapter 5</span><span>2 hours ago</span>
				</a>
			</article>
		</section>
		<section aria-labelledby="cw-novels">
			<a href="https://rinkocomics.com/novel/nov-a/">
				<img src="https://rinkocomics.com/n.webp" />
				<span class="line-clamp-2">Novel A</span>
			</a>
		</section>
	"#;
	let new_comics_html = r#"
		<div id="cwBrowse"><ul><li>
			<a href="https://rinkocomics.com/comic/new-c/">
				<img src="https://rinkocomics.com/nc.webp" />
				<span class="line-clamp-2">New Comic</span>
			</a>
		</li></ul></div>
	"#;
	let novel_home_html = r#"
		<section id="cw-latest">
			<article>
				<a class="line-clamp-2" href="https://rinkocomics.com/novel/lat-n/">Latest Novel</a>
				<img src="https://rinkocomics.com/ln.webp" />
				<a class="chapter-item" href="https://rinkocomics.com/novel_chapter/lat-n-ep-12/">
					<span>Episode 12</span><span>1 day ago</span>
				</a>
			</article>
		</section>
	"#;
	let home = Html::parse_with_url(home_html, BASE_URL).unwrap();
	let new_comics = Html::parse_with_url(new_comics_html, BASE_URL).unwrap();
	let novel_home = Html::parse_with_url(novel_home_html, BASE_URL).unwrap();
	let layout = build_home_layout(&home, &new_comics, &novel_home, false);

	let titles: Vec<_> = layout
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Editor's Choice",
			"Featured This Week",
			"Latest Releases (Comics)",
			"New Comics",
			"New Novels",
			"Latest Releases (Novels)",
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
	assert!(matches!(
		layout.components[2].value,
		HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			..
		}
	));
	assert!(matches!(
		layout.components[3].value,
		HomeComponentValue::Scroller { .. }
	));
	assert!(matches!(
		layout.components[4].value,
		HomeComponentValue::Scroller { .. }
	));
	assert!(matches!(
		layout.components[5].value,
		HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			..
		}
	));
}

#[aidoku_test]
fn review_locked_chapters_and_cw_pages_decoder() {
	let html_str = r##"
		<div id="cwChapters">
			<a class="chapter-item locked-chapter" data-reason="coin" data-post-id="99" href="#">
				<span class="chapter-number">Chapter 10</span>
			</a>
			<a class="chapter-item" href="https://rinkocomics.com/chapter/foo-ch-9/">
				<span class="chapter-number">Chapter 9</span>
				<span class="chapter-date">2 days ago</span>
			</a>
		</div>
	"##;
	let doc = Html::parse_with_url(html_str, BASE_URL).unwrap();
	let hidden = collect_chapters(&doc, html_str, BASE_URL, false);
	assert_eq!(hidden.len(), 1);
	assert!(!hidden[0].locked);
	assert_eq!(hidden[0].chapter_number, Some(9.0));

	let shown = collect_chapters(&doc, html_str, BASE_URL, true);
	assert_eq!(shown.len(), 2);
	assert!(shown[0].locked);
	assert_eq!(shown[0].chapter_number, Some(10.0));

	// Verify 16-byte XOR-masked base64 JSON decoding.
	let mask = [0x11_u8; 16];
	let plain = br#"["https://rinkocomics.com/p1.webp","https://rinkocomics.com/p2.webp"]"#;
	let mut payload = Vec::from(mask);
	for (i, b) in plain.iter().enumerate() {
		payload.push(b ^ mask[i % 16]);
	}
	let encoded = BASE64_STANDARD.encode(&payload);
	assert_eq!(
		decode_cw_pages(&encoded).unwrap(),
		[
			"https://rinkocomics.com/p1.webp",
			"https://rinkocomics.com/p2.webp"
		]
	);
}

#[aidoku_test]
fn live_home_browse_details_novels_and_pages() {
	defaults_set("showLocked", DefaultValue::Bool(false));
	let source = RinkoComics::new();

	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Editor's Choice",
			"Featured This Week",
			"Latest Releases (Comics)",
			"New Comics",
			"New Novels",
			"Latest Releases (Novels)",
		]
	);

	let browse = source.get_search_manga_list(None, 1, vec![]).unwrap();
	assert!(!browse.entries.is_empty());

	let dynamic = source.get_dynamic_filters().unwrap();
	assert_eq!(dynamic.len(), 1);

	let listing = source
		.get_manga_list(make_listing("new", "New Comics"), 1)
		.unwrap();
	assert!(!listing.entries.is_empty());

	// Live comic details, chapters, and pages.
	let comic = browse.entries[0].clone();
	let details = source.get_manga_update(comic, true, true).unwrap();
	assert!(!details.title.is_empty());
	assert!(details.cover.is_some());
	let chapters = details.chapters.as_ref().unwrap();
	assert!(!chapters.is_empty());
	assert!(chapters.iter().all(|c| !c.locked));
	let pages = source
		.get_page_list(details.clone(), chapters.last().unwrap().clone())
		.unwrap();
	assert!(!pages.is_empty());

	// Live novel details and prose episode.
	let novel = Manga {
		key: "novel/a-king-is-born-on-the-road".into(),
		title: String::new(),
		..Default::default()
	};
	let novel_details = source.get_manga_update(novel, true, true).unwrap();
	assert!(!novel_details.title.is_empty());
	let eps = novel_details.chapters.as_ref().unwrap();
	assert!(!eps.is_empty());
	let prose = source
		.get_page_list(novel_details.clone(), eps.last().unwrap().clone())
		.unwrap();
	assert_eq!(prose.len(), 1);

	assert!(matches!(
		source
			.handle_deep_link(format!("{BASE_URL}/comic/a-king-is-born-on-the-road/"))
			.unwrap(),
		Some(DeepLinkResult::Manga { key }) if key == "comic/a-king-is-born-on-the-road"
	));
}
