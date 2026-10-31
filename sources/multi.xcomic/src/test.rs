#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	MangaStatus,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;

// An English edition whose own record names no artist, though its work does
const EDITION_KEY: &str = "26yl6m";
// "On the Way to Meet Mom", whose English edition with the most chapters is xyvlkw
const TITLE_URL: &str = "https://xcomic.me/title/4wwyl-on-the-way-to-meet-mom";
const TITLE_EDITION: &str = "xyvlkw";

fn set_ratings(ratings: &[&str]) {
	defaults_set(
		"contentRatings",
		DefaultValue::StringArray(ratings.iter().map(|rating| (*rating).into()).collect()),
	);
}

fn search(query: Option<&str>, page: i32, filters: Vec<FilterValue>) -> MangaPageResult {
	XComic::new()
		.get_search_manga_list(query.map(Into::into), page, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: Vec::new(),
	}
}

#[aidoku_test]
fn browse_pages() {
	set_ratings(&["safe", "suggestive", "erotica", "pornographic"]);
	let first = search(None, 1, Vec::new());
	let second = search(None, 2, Vec::new());
	assert!(first.has_next_page);
	assert!(!first.entries.is_empty());
	assert!(
		second
			.entries
			.iter()
			.all(|manga| first.entries.iter().all(|seen| seen.key != manga.key))
	);
}

#[aidoku_test]
fn original_status_takes_several() {
	set_ratings(&["safe", "suggestive", "erotica", "pornographic"]);
	let filters = || {
		vec![
			FilterValue::Text {
				id: "year".into(),
				value: "2020".into(),
			},
			multi("genres", &["mecha"]),
		]
	};
	let mut both = filters();
	both.push(multi("original_status", &["releasing", "completed"]));
	let statuses: Vec<MangaStatus> = search(None, 1, both)
		.entries
		.into_iter()
		.map(|manga| manga.status)
		.collect();
	assert!(statuses.contains(&MangaStatus::Ongoing));
	assert!(statuses.contains(&MangaStatus::Completed));
	assert!(
		statuses
			.iter()
			.all(|status| matches!(status, MangaStatus::Ongoing | MangaStatus::Completed))
	);

	let mut hiatus = filters();
	hiatus.push(multi("original_status", &["hiatus"]));
	assert!(
		search(None, 1, hiatus)
			.entries
			.iter()
			.all(|manga| manga.status == MangaStatus::Hiatus)
	);
}

#[aidoku_test]
fn details_fall_back_to_the_work() {
	let manga = XComic::new()
		.get_manga_update(
			Manga {
				key: EDITION_KEY.into(),
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert!(manga.artists.unwrap().iter().any(|artist| artist == "Mon"));
	assert!(manga.tags.unwrap().iter().any(|tag| tag == "Longstrip"));
	assert!(manga.description.is_some());
}

#[aidoku_test]
fn chapters_and_pages() {
	let source = XComic::new();
	let manga = source
		.get_manga_update(
			Manga {
				key: EDITION_KEY.into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	let chapter = manga.chapters.clone().unwrap().pop().unwrap();
	let pages = source.get_page_list(manga, chapter).unwrap();
	assert!(!pages.is_empty());
}

#[aidoku_test]
fn title_links_open_the_browse_edition() {
	let link = XComic::new()
		.handle_deep_link(TITLE_URL.into())
		.unwrap()
		.unwrap();
	assert!(matches!(link, DeepLinkResult::Manga { key } if key == TITLE_EDITION));

	let pasted = search(Some(TITLE_URL), 1, Vec::new());
	assert_eq!(pasted.entries.len(), 1);
	assert_eq!(pasted.entries[0].key, TITLE_EDITION);
}
