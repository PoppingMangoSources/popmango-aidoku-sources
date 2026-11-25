#![expect(clippy::unwrap_used)]

use super::{
	Cocomic,
	helpers::{
		BASE_URL, abs_url, append_style_list, clean_chapter_title, is_valid_genre, listing,
		listing_date, parse_manga_chapter_path, swimlane_latest,
	},
};
use aidoku::{
	DeepLinkHandler, DynamicFilters, Home, HomeComponentValue, ListingProvider, Manga, Source,
	alloc::{String, Vec, format, vec},
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
	},
};
use aidoku_test::aidoku_test;
use madara::{Impl, Madara};

#[aidoku_test]
fn review_date_genre_chapter_and_locked_helpers() {
	assert!(listing_date("NEW").is_none());
	assert!(listing_date("5m ago").is_some());
	assert!(listing_date("2 days ago").is_some());
	assert!(is_valid_genre("Romance"));
	assert!(!is_valid_genre("Ch.133"));
	assert!(!is_valid_genre("Ch 134"));
	assert!(!is_valid_genre("Chapter 135"));
	assert!(!is_valid_genre("Publication: Ongoing"));
	assert!(!is_valid_genre("Bato Status: Complete"));
	assert!(clean_chapter_title("Chapter 136", Some(136.0)).is_none());
	assert!(clean_chapter_title("Ch. 136", Some(136.0)).is_none());
	assert_eq!(
		clean_chapter_title("Chapter 136 - The Duel", Some(136.0)).as_deref(),
		Some("The Duel")
	);
	assert_eq!(
		append_style_list("https://cocomic.co/manga/foo/chapter-1/"),
		"https://cocomic.co/manga/foo/chapter-1/?style=list"
	);
	assert_eq!(
		append_style_list("https://cocomic.co/manga/foo/chapter-1/?foo=1"),
		"https://cocomic.co/manga/foo/chapter-1/?foo=1&style=list"
	);
	assert_eq!(
		abs_url("/www/wwwroot/work_1/chap_1.webp").as_deref(),
		Some("https://cdn2.cocomic.co/work_1/chap_1.webp")
	);
	assert_eq!(
		parse_manga_chapter_path("https://cocomic.co/manga/foo/ep-2/"),
		Some((
			String::from("/manga/foo/"),
			Some(String::from("/manga/foo/ep-2/"))
		))
	);

	let home_fixture = Html::parse_with_url(
		r#"
		<div class="mobile-swimlane-section">
			<h2 class="swimlane-heading">Latest Updates</h2>
			<div class="mobile-swimlane-card">
				<a class="swimlane-title" href="https://cocomic.co/manga/hold-on/">Hold On</a>
				<img src="https://cocomic.co/c1.jpg" />
				<a class="swimlane-chap" href="https://cocomic.co/manga/hold-on/">Ch. 1</a>
			</div>
			<div class="mobile-swimlane-card">
				<a class="swimlane-title" href="https://cocomic.co/manga/real-one/">Real One</a>
				<img src="https://cocomic.co/c2.jpg" />
				<a class="swimlane-chap premium" href="https://cocomic.co/manga/real-one/chapter-9/">Ch. 9</a>
			</div>
			<div class="mobile-swimlane-card">
				<a class="swimlane-title" href="https://cocomic.co/manga/free-one/">Free One</a>
				<img src="https://cocomic.co/c3.jpg" />
				<a class="swimlane-chap" href="https://cocomic.co/manga/free-one/chapter-4/">Ch. 4</a>
			</div>
		</div>
		"#,
		BASE_URL,
	)
	.unwrap();
	let latest_hidden = swimlane_latest(&home_fixture, "Latest Updates", false);
	assert_eq!(latest_hidden.len(), 1);
	assert_eq!(latest_hidden[0].chapter.key, "/manga/free-one/chapter-4/");
	let latest_shown = swimlane_latest(&home_fixture, "Latest Updates", true);
	assert_eq!(latest_shown.len(), 2);
	assert!(latest_shown[0].chapter.locked);

	let impl_src = Cocomic::new();
	let params = impl_src.params();
	let ch_doc = Html::parse_with_url(
		r#"
		<ul>
			<li class="wp-manga-chapter premium">
				<span class="coin">15</span>
				<a href="https://cocomic.co/manga/foo/chapter-10/">Chapter 10</a>
			</li>
			<li class="wp-manga-chapter free-chap">
				<span class="coin free">Free</span>
				<a href="https://cocomic.co/manga/foo/chapter-9/">Chapter 9</a>
			</li>
		</ul>
		"#,
		BASE_URL,
	)
	.unwrap();
	defaults_set("showLocked", DefaultValue::Bool(false));
	let parsed_hidden: Vec<_> = ch_doc
		.select(&params.chapter_selector)
		.unwrap()
		.filter_map(|el| impl_src.parse_chapter_element(&params, el))
		.collect();
	assert_eq!(parsed_hidden.len(), 1);
	assert!(!parsed_hidden[0].locked);

	defaults_set("showLocked", DefaultValue::Bool(true));
	let parsed_shown: Vec<_> = ch_doc
		.select(&params.chapter_selector)
		.unwrap()
		.filter_map(|el| impl_src.parse_chapter_element(&params, el))
		.collect();
	assert_eq!(parsed_shown.len(), 2);
	assert!(parsed_shown[0].locked);
	defaults_set("showLocked", DefaultValue::Bool(false));
}

#[aidoku_test]
fn home_listings_search_details_and_pages() {
	defaults_set("showLocked", DefaultValue::Bool(false));
	let source = Madara::<Cocomic>::new();

	let home = source.get_home().unwrap();
	let titles: Vec<_> = home
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Trending",
			"Trending Now",
			"Latest Updates",
			"Popular Manga",
			"New Releases",
		]
	);
	if let HomeComponentValue::MangaChapterList { entries, .. } = &home.components[2].value {
		assert!(entries.iter().all(|e| e.chapter.url.is_none()));
		assert!(entries.iter().all(|e| !e.chapter.locked));
	}

	let list_page = source
		.get_manga_list(listing("latest", "Latest Updates"), 1)
		.unwrap();
	assert!(!list_page.entries.is_empty());
	assert!(list_page.has_next_page);

	let search = source
		.get_search_manga_list(Some("jinx".into()), 1, vec![])
		.unwrap();
	assert!(!search.entries.is_empty());

	let filters = source.get_dynamic_filters().unwrap();
	assert_eq!(filters.len(), 1);

	let trainer = Manga {
		key: "/manga/genius-martial-arts-trainer/".into(),
		title: String::new(),
		..Default::default()
	};
	let details = source.get_manga_update(trainer, true, true).unwrap();
	assert!(!details.title.is_empty());
	assert!(details.cover.is_some());
	let tags = details.tags.as_ref().unwrap();
	assert!(!tags.is_empty());
	assert!(
		tags.iter().all(|t| !t.starts_with("Ch.")),
		"chapter tags leaked into details: {tags:?}"
	);
	let chapters = details.chapters.as_ref().unwrap();
	assert!(!chapters.is_empty());
	assert!(chapters.iter().all(|c| !c.locked));
	assert!(
		chapters.iter().take(5).all(|c| {
			c.title
				.as_deref()
				.is_none_or(|t| !t.to_ascii_lowercase().starts_with("chapter "))
		}),
		"chapter title repeated chapter number"
	);

	let pages = source
		.get_page_list(details.clone(), chapters[0].clone())
		.unwrap();
	assert!(!pages.is_empty());

	let deep = source
		.handle_deep_link(format!("{BASE_URL}/manga/genius-martial-arts-trainer/"))
		.unwrap();
	assert!(deep.is_some());
}
