#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Manga, MangaPageResult,
	MangaStatus, PageContent, Viewer,
	alloc::{String, Vec, vec},
};
use aidoku_test::aidoku_test;

// The site's genre id for Action, from res/filters.json.
const GENRE: &str = "action";

fn source() -> Keyoapp<GenzToons> {
	Keyoapp::new()
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(String::from), 1, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: excluded.iter().map(|value| (*value).into()).collect(),
	}
}

fn details(manga: Manga) -> Manga {
	source().get_manga_update(manga, true, false).unwrap()
}

#[aidoku_test]
fn search_lists_the_catalogue_on_one_page() {
	let all = search(None, vec![]);
	assert!(all.entries.len() > 50);
	assert!(!all.has_next_page);
	// Cards carry only what the list shows.
	assert!(all.entries.iter().all(|manga| {
		manga.key.starts_with("/series/") && manga.cover.is_some() && manga.description.is_none()
	}));
}

#[aidoku_test]
fn search_finds_a_title() {
	let first = search(None, vec![]).entries.remove(0);
	// Letters and digits only: the cards' titles carry quotes the query needn't.
	let word = first
		.title
		.split(|c: char| !c.is_alphanumeric())
		.max_by_key(|word| word.len())
		.unwrap()
		.to_lowercase();
	let found = search(Some(&word), vec![]);
	assert!(found.entries.iter().any(|manga| manga.key == first.key));
	assert!(search(Some("zzqxwv"), vec![]).entries.is_empty());
}

#[aidoku_test]
fn genre_filter_includes_and_excludes() {
	let all = search(None, vec![]).entries.len();
	let action = search(None, vec![multi("genre", &[GENRE], &[])]).entries;
	let without = search(None, vec![multi("genre", &[], &[GENRE])]).entries;
	assert!(!action.is_empty() && action.len() < all);
	assert_eq!(action.len() + without.len(), all);
	let tags = details(action[0].clone()).tags.unwrap();
	assert!(tags.iter().any(|tag| tag.eq_ignore_ascii_case(GENRE)));
}

#[aidoku_test]
fn type_and_status_filters_narrow_results() {
	let all = search(None, vec![]).entries.len();
	let manga = search(None, vec![multi("type", &["manga"], &[])]).entries;
	assert!(!manga.is_empty() && manga.len() < all);
	assert_eq!(details(manga[0].clone()).viewer, Viewer::RightToLeft);
	let completed = search(None, vec![multi("status", &["completed"], &[])]).entries;
	assert!(!completed.is_empty() && completed.len() < all);
	assert_eq!(details(completed[0].clone()).status, MangaStatus::Completed);
}

#[aidoku_test]
fn details_chapters_and_pages_load() {
	let first = search(None, vec![]).entries.remove(0);
	let manga = source().get_manga_update(first, true, true).unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.cover.is_some());
	// Some series have no synopsis on the site at all.
	assert!(
		manga
			.description
			.as_deref()
			.is_none_or(|description| !description.is_empty())
	);
	assert!(manga.status != MangaStatus::Unknown);
	assert!(manga.tags.as_ref().is_some_and(|tags| !tags.is_empty()));
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty());
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.key.starts_with("/chapter/"))
	);
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.date_uploaded.is_some())
	);
	for chapter in [chapters.first(), chapters.last()].into_iter().flatten() {
		let pages = source()
			.get_page_list(manga.clone(), chapter.clone())
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
	let HomeComponentValue::Scroller { entries, .. } = &home.components[0].value else {
		panic!("the featured slider comes first");
	};
	assert!(home.components[0].title.is_none() && entries.len() >= 3);
	let titles: Vec<&str> = home
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	let position = |title: &str| titles.iter().position(|found| *found == title).unwrap();
	assert!(position("Latest Updates") < position("Trending"));
	assert!(position("Trending") < position("Recently Added"));
	let mut unique = titles.clone();
	unique.dedup();
	assert_eq!(unique.len(), titles.len(), "rows repeat");
	assert!(!titles.contains(&"Bulk Discounted Series"));
	let row = |title: &str| &home.components[position(title) + 1].value;
	assert!(
		matches!(row("Pinned Series"), HomeComponentValue::Scroller { entries, .. } if !entries.is_empty()),
		"pinned series is a card row"
	);
	let HomeComponentValue::BigScroller { entries: picks, .. } = row("Editor's Choice") else {
		panic!("editor's choice is a big scroller");
	};
	let pick = &picks[0];
	assert!(
		pick.cover.is_some()
			&& pick
				.description
				.as_deref()
				.is_some_and(|text| !text.is_empty())
	);
	assert!(
		search(None, vec![])
			.entries
			.iter()
			.any(|manga| manga.key == pick.key)
	);
	let HomeComponentValue::MangaChapterList {
		page_size, entries, ..
	} = row("Latest Updates")
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
	let manga = source()
		.get_manga_update(first.clone(), false, true)
		.unwrap();
	let chapter = manga.chapters.unwrap().remove(0);
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
	// The catalogue itself is no series.
	for path in ["/series/", "/series/?genre=action"] {
		assert_eq!(
			source()
				.handle_deep_link(format!("{BASE_URL}{path}"))
				.unwrap(),
			None
		);
	}
}

#[aidoku_test]
fn an_unknown_series_fails() {
	let manga = Manga {
		key: "/series/zzqxwv-no-such-series/".into(),
		..Default::default()
	};
	let result = source().get_manga_update(manga, true, true);
	assert!(result.map_or(true, |manga| manga.chapters.unwrap_or_default().is_empty()));
}
