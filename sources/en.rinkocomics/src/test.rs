#![expect(clippy::unwrap_used)]

use super::{
	RinkoComics,
	helpers::{
		BASE_URL, SORTS, chapter, decode_pages, display_title, key_of, parse_date, parse_status,
		slugify,
	},
	home::layout,
};
use aidoku::{
	DeepLinkHandler, DeepLinkResult, DynamicFilters, FilterValue, Home, HomeComponentValue,
	Listing, ListingKind, ListingProvider, Manga, MangaStatus, Source,
	alloc::{String, Vec, format, string::ToString, vec},
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
		std::current_date,
	},
};
use aidoku_test::aidoku_test;
use base64::prelude::*;

fn row(html: &str) -> Option<aidoku::Chapter> {
	let doc = Html::parse_fragment_with_url(html, BASE_URL).unwrap();
	chapter(&doc.select_first("a").unwrap())
}

#[aidoku_test]
fn review_chapter_rows() {
	let comic = row(
		r#"<a class="chapter-item" href="https://rinkocomics.com/chapter/28-youth-chapter-12/">
		<span class="flex"><span class="text-sm font-semibold"> Chapter 12 </span>
		<span class="text-xs">Sep 27, 2026 · 38 views</span></span></a>"#,
	)
	.unwrap();
	assert_eq!(comic.key, "chapter/28-youth-chapter-12");
	assert_eq!(comic.chapter_number, Some(12.0));
	assert_eq!(comic.title, None);
	assert!(!comic.locked);

	// Paid rows keep their address in data-permalink and end with a "Free on" date.
	let paid = row(r##"<a class="chapter-item" data-permalink="https://rinkocomics.com/chapter/baby-dragon-chapter-22/" data-reason="login_required" href="#">
		<span class="flex"><span class="font-semibold"> Chapter 22 <span aria-label="Paid"><svg></svg></span></span>
		<span class="text-xs">Nov 27, 2026 · 0 views</span></span>
		<span>Free on <span>Dec 27</span></span></a>"##)
	.unwrap();
	assert_eq!(paid.key, "chapter/baby-dragon-chapter-22");
	assert!(paid.locked);

	let episode = row(r#"<a href="https://rinkocomics.com/novel_chapter/a-beast-chapter-50/">
		<span class="flex"><span class="font-semibold">#50</span><span class="truncate">Crown Prince Iovet (2)</span></span>
		<span class="text-xs">Apr 18, 2026</span></a>"#)
	.unwrap();
	assert_eq!(episode.key, "novel_chapter/a-beast-chapter-50");
	assert_eq!(episode.chapter_number, Some(50.0));
	assert_eq!(episode.title.as_deref(), Some("Crown Prince Iovet (2)"));

	// Home rows give an age; paid ones there have no address at all.
	let home = row(r#"<a class="chapter-item" href="https://rinkocomics.com/chapter/x-chapter-20/"><span>Ch. 20</span><span>7h</span></a>"#)
		.unwrap();
	assert_eq!(home.chapter_number, Some(20.0));
	let age = current_date() - home.date_uploaded.unwrap();
	assert!((7 * 3600..7 * 3600 + 60).contains(&age));
	assert!(
		row(r##"<a class="chapter-item" data-reason="login_required" href="#"><span>Ch. 22<span></span></span><span>7h</span></a>"##)
			.is_none()
	);
}

#[aidoku_test]
fn review_dates_statuses_and_slugs() {
	// "Sep 27, 2026" parses only in the app: the runner needs a time in the format.
	for label in ["1w", "1mo", "1d", "3h", "12m", "2y"] {
		assert!(parse_date(label).is_some(), "{label}");
	}
	for label in ["#50", "Chapter 12", "Dec 27", "2nd Love", ""] {
		assert_eq!(parse_date(label), None, "{label}");
	}
	assert_eq!(parse_status("Season end"), MangaStatus::Hiatus);
	assert_eq!(parse_status("Dropped"), MangaStatus::Cancelled);
	defaults_set("labelNovels", DefaultValue::Bool(true));
	assert_eq!(display_title("Flowers", "novel/flowers"), "Flowers (Novel)");
	// The site's own "[Novel]" suffix is replaced, not doubled.
	assert_eq!(
		display_title("Don't Tame It [Novel]", "novel/dont-tame-it"),
		"Don't Tame It (Novel)"
	);
	assert_eq!(display_title("Youth", "comic/youth"), "Youth");
	defaults_set("labelNovels", DefaultValue::Bool(false));
	assert_eq!(display_title("Flowers", "novel/flowers"), "Flowers");
	assert_eq!(
		display_title("Don't Tame It [Novel]", "novel/dont-tame-it"),
		"Don't Tame It"
	);
	defaults_set("labelNovels", DefaultValue::Bool(true));
	assert_eq!(slugify("Slice of Life"), "slice-of-life");
	assert_eq!(slugify("Survival Game"), "survival-game");
	assert_eq!(
		key_of(&format!("{BASE_URL}/comic/28-youth/")).as_deref(),
		Some("comic/28-youth")
	);
	assert_eq!(
		key_of(&format!("{BASE_URL}/comic/")).as_deref(),
		Some("comic")
	);
	assert_eq!(key_of("https://example.com/comic/x/"), None);
	assert_eq!(SORTS, ["az", "updated", "views", "new"]);
}

#[aidoku_test]
fn review_page_list_decoder() {
	let key = [0x11_u8; 16];
	let plain = br#"["https://rinkocomics.com/p1.webp","https://rinkocomics.com/p2.webp"]"#;
	let mut payload = Vec::from(key);
	payload.extend(plain.iter().enumerate().map(|(i, b)| b ^ key[i % 16]));
	assert_eq!(
		decode_pages(&format!(" {} ", BASE64_STANDARD.encode(&payload))).unwrap(),
		[
			"https://rinkocomics.com/p1.webp",
			"https://rinkocomics.com/p2.webp"
		]
	);
	assert!(decode_pages(&BASE64_STANDARD.encode([0_u8; 8])).is_err());
}

#[aidoku_test]
fn review_home_order_and_shapes() {
	let home = Html::parse_with_url(
		r##"<div data-cw-slider><div data-slide>
			<div><span class="chip">Romance</span><span class="chip">Fantasy</span><span>51 chapters</span></div>
			<h2>Hero A</h2><img src="https://rinkocomics.com/a.webp"><p>A summary.</p>
			<a class="btn-primary" href="https://rinkocomics.com/comic/hero-a/">Read</a>
		</div></div>
		<section aria-labelledby="cw-week-m"><a href="https://rinkocomics.com/comic/week-a/"><img src="https://rinkocomics.com/w.webp"><span class="line-clamp-2">Week A</span></a></section>
		<section id="cw-latest"><article>
			<a class="line-clamp-2" href="https://rinkocomics.com/comic/lat-c/">Latest Comic</a><img src="https://rinkocomics.com/lc.webp">
			<a class="chapter-item" data-reason="login_required" href="#"><span>Ch. 6</span><span>1h</span></a>
			<a class="chapter-item" href="https://rinkocomics.com/chapter/lat-c-chapter-5/"><span>Ch. 5</span><span>7h</span></a>
		</article></section>
		<section aria-labelledby="cw-novels">
			<a href="https://rinkocomics.com/novel/nov-a/"><img src="https://rinkocomics.com/n.webp"><span class="line-clamp-2">Novel A</span></a>
			<a href="https://rinkocomics.com/novel/">View all</a>
		</section>"##,
		BASE_URL,
	)
	.unwrap();
	let new = Html::parse_with_url(
		r#"<div id="cwBrowse"><ul><li><a href="https://rinkocomics.com/comic/new-c/"><img src="https://rinkocomics.com/nc.webp"><span class="line-clamp-2">New Comic</span></a></li></ul></div>"#,
		BASE_URL,
	)
	.unwrap();
	let novels = Html::parse_with_url(
		r#"<section id="cw-latest"><article>
			<a class="line-clamp-2" href="https://rinkocomics.com/novel/lat-n/">Latest Novel</a>
			<a class="chapter-item" href="https://rinkocomics.com/novel_chapter/lat-n-chapter-12/"><span>Episode 12</span><span>3w</span></a>
		</article></section>"#,
		BASE_URL,
	)
	.unwrap();
	let built = layout(Some(&home), Some(&new), Some(&novels));
	let titles: Vec<_> = built
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
	let HomeComponentValue::BigScroller { entries, .. } = &built.components[0].value else {
		panic!("Editor's Choice must be a big scroller");
	};
	assert_eq!(entries[0].key, "comic/hero-a");
	assert_eq!(
		entries[0].tags,
		Some(vec![String::from("Fantasy"), String::from("Romance")])
	);
	let HomeComponentValue::MangaChapterList {
		entries, page_size, ..
	} = &built.components[2].value
	else {
		panic!("Latest Releases (Comics) must be a chapter list");
	};
	assert_eq!(*page_size, Some(5));
	assert_eq!(entries[0].chapter.key, "chapter/lat-c-chapter-5");
	let HomeComponentValue::Scroller { entries, .. } = &built.components[4].value else {
		panic!("New Novels must be a scroller");
	};
	assert_eq!(entries.len(), 1);

	// A failed page leaves its sections out.
	let partial = layout(None, Some(&new), None);
	assert_eq!(partial.components.len(), 1);
}

#[aidoku_test]
fn live_home_and_listings() {
	let source = RinkoComics::new();
	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(titles.len(), 6, "{titles:?}");
	for (id, name) in [
		("updated", "Latest"),
		("views", "Views"),
		("new", "New"),
		("novels", "Novels"),
		("novels-updated", "Novels updated"),
	] {
		let listing = Listing {
			id: id.into(),
			name: name.into(),
			kind: ListingKind::Default,
		};
		assert!(
			!source
				.get_manga_list(listing, 1)
				.unwrap()
				.entries
				.is_empty(),
			"{id}"
		);
	}
}

#[aidoku_test]
fn live_search_sorts_and_filters() {
	let source = RinkoComics::new();
	let first = source.get_search_manga_list(None, 1, vec![]).unwrap();
	assert!(first.has_next_page);
	let second = source.get_search_manga_list(None, 2, vec![]).unwrap();
	assert_ne!(first.entries[0].key, second.entries[0].key);
	for index in 0..SORTS.len() as i32 {
		let sort = FilterValue::Sort {
			id: "sort".into(),
			index,
			ascending: false,
		};
		assert!(
			!source
				.get_search_manga_list(None, 1, vec![sort])
				.unwrap()
				.entries
				.is_empty()
		);
	}
	let dropped = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "status".into(),
				value: "dropped".into(),
			}],
		)
		.unwrap();
	assert!(
		dropped
			.entries
			.iter()
			.all(|m| m.status == MangaStatus::Cancelled)
	);

	// A tapped tag and the genre filter find the same titles.
	let tapped = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::Select {
				id: "genre".into(),
				value: "Slice of Life".into(),
			}],
		)
		.unwrap();
	let picked = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genres[]".into(),
				included: vec!["slice-of-life".into()],
				excluded: vec![],
			}],
		)
		.unwrap();
	assert!(!tapped.entries.is_empty());
	let keys = |page: &aidoku::MangaPageResult| {
		page.entries
			.iter()
			.map(|m| m.key.clone())
			.collect::<Vec<_>>()
	};
	assert_eq!(keys(&tapped), keys(&picked));
	assert_ne!(keys(&tapped), keys(&first));

	let novels = source
		.get_search_manga_list(
			Some("flower".into()),
			1,
			vec![FilterValue::Select {
				id: "type".into(),
				value: "novel".into(),
			}],
		)
		.unwrap();
	assert!(novels.entries.iter().all(|m| m.key.starts_with("novel/")));

	assert_eq!(source.get_dynamic_filters().unwrap().len(), 1);
}

#[aidoku_test]
fn live_details_chapters_and_pages() {
	defaults_set("showLocked", DefaultValue::Bool(false));
	let source = RinkoComics::new();
	// 179 chapters, 20 on the page and the rest through load-more.
	let long = source
		.get_manga_update(
			Manga {
				key: "comic/another-typical-fantasy-romance".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(long.title, "Another Typical Fantasy Romance");
	assert!(long.cover.is_some() && long.description.is_some() && long.tags.is_some());
	let chapters = long.chapters.unwrap();
	assert!(chapters.len() > 150, "{}", chapters.len());
	assert!(chapters.iter().all(|c| !c.locked));
	assert!(
		chapters
			.windows(2)
			.all(|w| w[0].chapter_number >= w[1].chapter_number)
	);
	let pages = source
		.get_page_list(Manga::default(), chapters.last().unwrap().clone())
		.unwrap();
	assert!(!pages.is_empty());

	let novel = source
		.get_manga_update(
			Manga {
				key: "novel/a-beast-swallowed-by-a-flower".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert!(novel.tags.is_some(), "novel genres");
	let episodes = novel.chapters.unwrap();
	assert!(episodes.iter().all(|c| !c.locked));
	let text = source
		.get_page_list(Manga::default(), episodes.last().unwrap().clone())
		.unwrap();
	assert_eq!(text.len(), 1);

	defaults_set("showLocked", DefaultValue::Bool(true));
	let all = source
		.get_manga_update(
			Manga {
				key: "novel/a-beast-swallowed-by-a-flower".into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap()
		.chapters
		.unwrap();
	assert!(all.len() > episodes.len());
	let locked = all.iter().find(|c| c.locked).unwrap();
	assert!(locked.key.starts_with("novel_chapter/"));
	assert!(
		source
			.get_page_list(Manga::default(), locked.clone())
			.is_err()
	);
	defaults_set("showLocked", DefaultValue::Bool(false));
}

#[aidoku_test]
fn live_deep_links() {
	let source = RinkoComics::new();
	assert!(matches!(
		source.handle_deep_link(format!("{BASE_URL}/comic/28-youth/")).unwrap(),
		Some(DeepLinkResult::Manga { key }) if key == "comic/28-youth"
	));
	assert!(matches!(
		source.handle_deep_link(format!("{BASE_URL}/chapter/28-youth-chapter-12/")).unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, key })
			if manga_key == "comic/28-youth" && key == "chapter/28-youth-chapter-12"
	));
	assert!(matches!(
		source
			.handle_deep_link(format!("{BASE_URL}/novel_chapter/a-beast-swallowed-by-a-flower-chapter-1/"))
			.unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, .. }) if manga_key == "novel/a-beast-swallowed-by-a-flower"
	));
	assert!(
		source
			.handle_deep_link("https://example.com/comic/x/".to_string())
			.unwrap()
			.is_none()
	);
}
