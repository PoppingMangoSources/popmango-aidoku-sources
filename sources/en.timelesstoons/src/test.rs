#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Manga,
	MangaPageResult, MangaStatus, PageContent, Viewer,
	alloc::{String, Vec, format, vec},
};
use aidoku_test::aidoku_test;

// The site's two most common genres, its most common type and a status, as res/filters.json ids
const GENRE: &str = "fantasy";
const SECOND_GENRE: &str = "drama";
const TYPE: &str = "manga";
const STATUS: &str = "ongoing";

fn source() -> Keyoapp<TimelessToons> {
	Keyoapp::new()
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(String::from), 1, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: Vec::new(),
	}
}

fn details(manga: Manga) -> Manga {
	source().get_manga_update(manga, true, false).unwrap()
}

#[aidoku_test]
fn review_filters_are_include_only() {
	// As on the site, no filter can exclude
	let filters = include_str!("../res/filters.json");
	assert_eq!(filters.matches("\"canExclude\": false").count(), 3);
	assert!(!filters.contains("\"canExclude\": true"));
}

#[aidoku_test]
fn search_lists_the_catalogue_on_one_page() {
	let all = search(None, vec![]);
	assert!(all.entries.len() > 10);
	assert!(!all.has_next_page);
	// Cards carry only what the list shows
	assert!(all.entries.iter().all(|manga| {
		manga.key.starts_with("/series/") && manga.cover.is_some() && manga.description.is_none()
	}));
}

#[aidoku_test]
fn search_finds_a_title() {
	let first = search(None, vec![]).entries.remove(0);
	// Letters and digits only: the cards' titles carry quotes the query needn't
	let word = first
		.title
		.split(|c: char| !c.is_alphanumeric())
		.max_by_key(|word| word.len())
		.unwrap()
		.to_lowercase();
	let found = search(Some(&word), vec![]);
	assert!(found.entries.iter().any(|manga| manga.key == first.key));
	assert!(search(Some("zzqxwv"), vec![]).entries.is_empty());
	// A pasted link finds that series alone
	let pasted = search(Some(&format!("{BASE_URL}{}", first.key)), vec![]);
	assert_eq!(pasted.entries, [first]);
}

#[aidoku_test]
fn genre_filter_needs_every_genre() {
	let all = search(None, vec![]).entries.len();
	let one = search(None, vec![multi("genre", &[GENRE])]).entries;
	assert!(!one.is_empty() && one.len() < all);
	// As on the site, a series needs every chosen genre
	let both = search(None, vec![multi("genre", &[GENRE, SECOND_GENRE])]).entries;
	assert!(both.len() < one.len());
	assert!(both.iter().all(|manga| one.contains(manga)));
}

#[aidoku_test]
fn type_and_status_filters_narrow_results() {
	let all = search(None, vec![]).entries.len();
	let typed = search(None, vec![multi("type", &[TYPE])]).entries;
	assert!(!typed.is_empty() && typed.len() <= all);
	assert_eq!(details(typed[0].clone()).viewer, Viewer::RightToLeft);
	let by_status = search(None, vec![multi("status", &[STATUS])]).entries;
	assert!(!by_status.is_empty() && by_status.len() < all);
	assert_eq!(details(by_status[0].clone()).status, MangaStatus::Ongoing);
}

#[aidoku_test]
fn details_chapters_and_pages_load() {
	let first = search(None, vec![]).entries.remove(0);
	let manga = source().get_manga_update(first, true, true).unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.cover.is_some());
	// Some series have no synopsis on the site at all
	assert!(
		manga
			.description
			.as_deref()
			.is_none_or(|description| !description.is_empty())
	);
	assert!(manga.status != MangaStatus::Unknown);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty());
	// The runner can't parse a date without a time, so "Aug 3, 2026" dates are checked in the app
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.key.starts_with("/chapter/"))
	);
	// Newest and oldest chapters, read with only their keys as the app may pass them
	for chapter in [chapters.first(), chapters.last()].into_iter().flatten() {
		let pages = source()
			.get_page_list(
				Manga {
					key: manga.key.clone(),
					..Default::default()
				},
				Chapter {
					key: chapter.key.clone(),
					..Default::default()
				},
			)
			.unwrap();
		assert!(!pages.is_empty());
		assert!(pages.iter().all(
			|page| matches!(&page.content, PageContent::Url(url, _) if url.starts_with("https://"))
		));
	}
}

#[aidoku_test]
fn home_follows_the_site() {
	let home = source().get_home().unwrap();
	let titles: Vec<&str> = home
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	let mut unique = titles.clone();
	unique.dedup();
	assert_eq!(unique.len(), titles.len(), "rows repeat");
	assert!(!titles.contains(&"Bulk Discounted Series"));
	let latest = home
		.components
		.iter()
		.find(|component| component.title.as_deref() == Some("Latest Updates"))
		.unwrap();
	let HomeComponentValue::MangaChapterList {
		page_size, entries, ..
	} = &latest.value
	else {
		panic!("latest updates is a chapter list");
	};
	assert_eq!(*page_size, Some(5));
	let entry = &entries[0];
	let chapters = source()
		.get_manga_update(entry.manga.clone(), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.key == entry.chapter.key)
	);
}

#[aidoku_test]
fn deep_links_open_series_and_chapters() {
	let first = search(None, vec![]).entries.remove(0);
	let chapter = source()
		.get_manga_update(first.clone(), false, true)
		.unwrap()
		.chapters
		.unwrap()
		.remove(0);
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}{}", first.key))
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: first.key.clone()
		})
	);
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}{}", chapter.key))
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: first.key,
			key: chapter.key
		})
	);
	// The catalogue itself is no series
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}/series/"))
			.unwrap(),
		None
	);
}

#[aidoku_test]
fn an_unknown_series_fails() {
	let manga = Manga {
		key: "/series/zzqxwv-no-such-series/".into(),
		..Default::default()
	};
	assert!(
		source()
			.get_manga_update(manga.clone(), true, true)
			.is_err()
	);
	assert!(source().get_manga_update(manga, false, true).is_err());
}
