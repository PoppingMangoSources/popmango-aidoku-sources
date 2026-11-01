#![expect(clippy::unwrap_used, clippy::panic)]

use super::*;
use aidoku::{
	ContentRating, LinkValue, MangaStatus, Viewer,
	alloc::{Vec, vec},
	imports::net::Request,
};
use aidoku_test::aidoku_test;
use helpers::locked_chapters;
use models::{ChapterItem, Unlock};

// A manhwa, and a novel whose id is also a different series' id.
const SERIES_KEY: &str = "/series/165";
const SERIES_CHAPTER_KEY: &str = "/d8eb658e3a903420";
const NOVEL_KEY: &str = "/novel/12";

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	FlameComics::new()
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: excluded.iter().map(|value| (*value).into()).collect(),
	}
}

fn sort(index: i32, ascending: bool) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending,
	}
}

fn manga(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

fn chapter_item(number: &str, price: Option<&str>, public_release: Option<i64>) -> ChapterItem {
	ChapterItem {
		chapter: number.into(),
		title: None,
		token: number.into(),
		release_date: None,
		price: price.map(Into::into),
		public_release,
	}
}

#[aidoku_test]
fn browse_lists_series_and_novels_with_titles_and_covers() {
	let result = search(None, vec![]);
	assert!(result.entries.len() > 100);
	assert!(!result.has_next_page);
	assert!(
		result
			.entries
			.iter()
			.all(|m| !m.title.is_empty() && m.cover.is_some())
	);
	let keys: Vec<&str> = result.entries.iter().map(|m| m.key.as_str()).collect();
	assert!(keys.contains(&SERIES_KEY) && keys.contains(&NOVEL_KEY));
	// Cards carry key, title and cover only.
	assert!(
		result
			.entries
			.iter()
			.all(|m| m.description.is_none() && m.tags.is_none())
	);
}

#[aidoku_test]
fn title_query_narrows() {
	let result = search(Some("30 years have passed"), vec![]);
	assert_eq!(result.entries.len(), 1);
	assert_eq!(result.entries[0].key, SERIES_KEY);
	assert!(
		search(Some("zzqq no such title"), vec![])
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn title_sort_follows_the_arrow() {
	let titles = |ascending| -> Vec<String> {
		search(None, vec![sort(0, ascending)])
			.entries
			.into_iter()
			.map(|m| m.title.to_lowercase())
			.collect()
	};
	let ascending = titles(true);
	assert!(ascending.windows(2).all(|pair| pair[0] <= pair[1]));
	let mut descending = titles(false);
	descending.reverse();
	assert_eq!(ascending, descending);
}

#[aidoku_test]
fn popularity_sort_puts_the_top_ranked_first() {
	let first = search(None, vec![sort(1, true)]).entries.remove(0);
	let last = search(None, vec![sort(1, false)]).entries.remove(0);
	assert_ne!(first.key, last.key);
}

#[aidoku_test]
fn genre_filter_includes_and_excludes() {
	let academy = search(None, vec![multi("genre", &["Academy"], &[])]);
	assert!(academy.entries.iter().any(|m| m.key == SERIES_KEY));
	let without = search(None, vec![multi("genre", &[], &["Academy"])]);
	assert!(!without.entries.iter().any(|m| m.key == SERIES_KEY));
	let all = search(None, vec![]).entries.len();
	assert_eq!(academy.entries.len() + without.entries.len(), all);
}

#[aidoku_test]
fn type_filter_selects_novels() {
	let novels = search(None, vec![multi("type", &["Novel", "Web Novel"], &[])]);
	assert!(!novels.entries.is_empty());
	assert!(novels.entries.iter().all(|m| m.key.starts_with("/novel/")));
	let manhwa = search(None, vec![multi("type", &["Manhwa"], &[])]);
	assert!(manhwa.entries.iter().all(|m| m.key.starts_with("/series/")));
}

#[aidoku_test]
fn status_and_year_filters_narrow() {
	let ongoing = search(None, vec![multi("status", &["Ongoing"], &[])]);
	let completed = search(None, vec![multi("status", &["Completed"], &[])]);
	assert!(ongoing.entries.iter().any(|m| m.key == SERIES_KEY));
	assert!(!completed.entries.iter().any(|m| m.key == SERIES_KEY));
	let year = search(None, vec![multi("year", &["2026"], &[])]);
	assert!(year.entries.iter().any(|m| m.key == SERIES_KEY));
	let other = search(None, vec![multi("year", &["2010"], &[])]);
	assert!(!other.entries.iter().any(|m| m.key == SERIES_KEY));
}

#[aidoku_test]
fn series_details_and_chapters() {
	let manga = FlameComics::new()
		.get_manga_update(manga(SERIES_KEY), true, true)
		.unwrap();
	assert_eq!(manga.title, "30 Years Have Passed Since the Prologue");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert_eq!(manga.content_rating, ContentRating::Safe);
	assert_eq!(manga.authors, Some(vec!["Markellaha".into()]));
	assert_eq!(manga.artists, Some(vec!["Studio GreenKirin".into()]));
	assert!(manga.tags.unwrap().contains(&"Academy".into()));
	assert!(manga.description.unwrap().contains("transmigrated"));
	assert_eq!(
		manga.url.as_deref(),
		Some("https://flamecomics.xyz/series/165")
	);
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() >= 22);
	assert!(chapters.iter().all(|c| c.key.starts_with('/') && !c.locked));
	assert!(
		chapters
			.iter()
			.any(|c| c.key == SERIES_CHAPTER_KEY && c.chapter_number == Some(22.0))
	);
	// Newest first, as the site lists them.
	assert!(chapters[0].chapter_number >= chapters[1].chapter_number);
}

#[aidoku_test]
fn chapters_only_update_leaves_details() {
	let manga = FlameComics::new()
		.get_manga_update(manga(SERIES_KEY), false, true)
		.unwrap();
	assert!(manga.title.is_empty() && manga.description.is_none());
	assert!(!manga.chapters.unwrap().is_empty());
}

#[aidoku_test]
fn novel_and_series_with_one_id_stay_apart() {
	let novel = FlameComics::new()
		.get_manga_update(manga(NOVEL_KEY), true, false)
		.unwrap();
	let series = FlameComics::new()
		.get_manga_update(manga("/series/12"), true, false)
		.unwrap();
	assert_eq!(novel.title, "I'm a Young God, Won't you Raise Me?");
	assert_eq!(series.title, "A Returner's Magic Should Be Special");
	assert_eq!(novel.viewer, Viewer::Unknown);
}

#[aidoku_test]
fn novel_early_access_chapters_are_locked() {
	let chapters = FlameComics::new()
		.get_manga_update(manga(NOVEL_KEY), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(chapters.len() > 20);
	assert!(chapters[0].locked);
	assert!(!chapters.last().unwrap().locked);
}

// The site renders each locked chapter link with data-locked="true".
#[aidoku_test]
fn novel_locks_match_the_site() {
	let page = Request::get(format!("{BASE_URL}{NOVEL_KEY}"))
		.unwrap()
		.html()
		.unwrap();
	let mut rendered: Vec<String> = page
		.select("a[data-locked=true]")
		.unwrap()
		.filter_map(|link| link.attr("href"))
		.filter_map(|href| href.rsplit('/').next().map(|token| format!("/{token}")))
		.collect();
	let mut ours: Vec<String> = FlameComics::new()
		.get_manga_update(manga(NOVEL_KEY), false, true)
		.unwrap()
		.chapters
		.unwrap()
		.into_iter()
		.filter(|chapter| chapter.locked)
		.map(|chapter| chapter.key)
		.collect();
	rendered.sort();
	ours.sort();
	assert!(!ours.is_empty());
	assert_eq!(ours, rendered);
}

#[aidoku_test]
fn series_pages_come_from_the_cdn() {
	let pages = FlameComics::new()
		.get_page_list(
			manga(SERIES_KEY),
			Chapter {
				key: SERIES_CHAPTER_KEY.into(),
				..Default::default()
			},
		)
		.unwrap();
	assert_eq!(pages.len(), 16);
	let PageContent::Url(first, _) = &pages[0].content else {
		panic!("expected an image url");
	};
	assert_eq!(
		first,
		"https://cdn.flamecomics.xyz/uploads/images/series/165/d8eb658e3a903420/30YRS-22-00.webp?1793279454"
	);
}

#[aidoku_test]
fn novel_chapters_read_as_text_or_fail_when_locked() {
	let source = FlameComics::new();
	let chapters = source
		.get_manga_update(manga(NOVEL_KEY), false, true)
		.unwrap()
		.chapters
		.unwrap();
	let free = chapters.last().unwrap().clone();
	let pages = source.get_page_list(manga(NOVEL_KEY), free).unwrap();
	let PageContent::Text(text) = &pages[0].content else {
		panic!("expected text");
	};
	assert!(text.contains("Chapter 1"));
	let locked = chapters[0].clone();
	assert!(source.get_page_list(manga(NOVEL_KEY), locked).is_err());
}

#[aidoku_test]
fn unknown_key_fails() {
	assert!(
		FlameComics::new()
			.get_manga_update(manga("/series/999999"), true, true)
			.is_err()
	);
}

#[aidoku_test]
fn home_puts_the_banner_first_and_names_latest_rows() {
	let layout = FlameComics::new().get_home().unwrap();
	let titles: Vec<&str> = layout
		.components
		.iter()
		.filter_map(|c| c.title.as_deref())
		.collect();
	assert_eq!(
		titles,
		[
			"Featured",
			"Popular",
			"Latest Updates",
			"Novels",
			"Staff Picks",
			"Latest Novels"
		]
	);
	let HomeComponentValue::ImageScroller { links, .. } = &layout.components[0].value else {
		panic!("expected the banner first");
	};
	assert!(links.iter().all(|link| matches!(
		&link.value,
		Some(LinkValue::Manga(m)) if m.key.starts_with("/series/") || m.key.starts_with("/novel/")
	)));
	let HomeComponentValue::Scroller { entries, .. } = &layout.components[3].value else {
		panic!("expected the novels row");
	};
	assert!(entries.iter().all(|link| matches!(
		&link.value,
		Some(LinkValue::Manga(m)) if m.key.starts_with("/novel/")
	)));
	// The reported bug: every latest row read "Untitled".
	for index in [2, 5] {
		let HomeComponentValue::MangaChapterList {
			entries, page_size, ..
		} = &layout.components[index].value
		else {
			panic!("expected a latest row");
		};
		assert_eq!(*page_size, Some(3));
		assert!(!entries.is_empty());
		assert!(
			entries
				.iter()
				.all(|e| !e.manga.title.is_empty() && e.chapter.chapter_number.is_some())
		);
	}
}

#[aidoku_test]
fn review_deep_links_keep_the_site_paths() {
	let source = FlameComics::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert!(matches!(
		link("https://flamecomics.xyz/series/165"),
		Some(DeepLinkResult::Manga { key }) if key == SERIES_KEY
	));
	assert!(matches!(
		link("https://flamecomics.xyz/novel/12/d366f754a38b450b?x=1"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == NOVEL_KEY && key == "/d366f754a38b450b"
	));
	assert!(link("https://flamecomics.xyz/browse").is_none());
	assert!(link("https://example.com/series/1").is_none());
}

#[aidoku_test]
fn review_rolling_policy_locks_the_newest_priced_chapters() {
	let chapters: Vec<ChapterItem> = (1..=30)
		.rev()
		.map(|n| chapter_item(&format!("{n}.00"), Some("50.00"), None))
		.collect();
	let unlock = Unlock {
		unlock_policy: Some("ROLLING_LATEST".into()),
		locked_chapter_count: Some(20),
		locked_chapter_percent: Some(10),
		locked_chapter_max: None,
	};
	let locks = locked_chapters(&unlock, &chapters, 0);
	assert_eq!(locks.iter().filter(|locked| **locked).count(), 20);
	assert!(locks[0] && !locks[29]);
	// Reversed input locks the same chapter numbers.
	let mut reversed = chapters;
	reversed.reverse();
	let reversed_locks = locked_chapters(&unlock, &reversed, 0);
	assert!(reversed_locks[29] && !reversed_locks[0]);
}

#[aidoku_test]
fn review_markdown_keeps_emphasis_and_escapes_text() {
	assert_eq!(
		helpers::markdown(
			"<p><br /></p>\n<p>He said <em>no </em>twice.</p><p><strong>Chapter 1</strong></p><p>* * *</p>"
		),
		"He said *no* twice\\.\n\n**Chapter 1**\n\n\\* \\* \\*"
	);
	assert_eq!(
		helpers::markdown("A bare description."),
		"A bare description\\."
	);
	assert_eq!(helpers::markdown("<p> </p>"), "");
}

#[aidoku_test]
fn review_release_times_and_free_chapters_are_public() {
	let unlock = Unlock::default();
	let chapters = [
		chapter_item("3.00", Some("50.00"), Some(200)),
		chapter_item("2.00", Some("50.00"), Some(50)),
		chapter_item("1.00", None, None),
	];
	assert_eq!(
		locked_chapters(&unlock, &chapters, 100),
		[true, false, false]
	);
}
