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

// The site matches formats against its own category list, which details don't carry
#[aidoku_test]
fn formats_narrow_like_genres() {
	let keys = |id: &str, included: &[&str], excluded: &[&str]| -> Vec<String> {
		let filter = FilterValue::MultiSelect {
			id: id.into(),
			included: included.iter().map(|value| (*value).into()).collect(),
			excluded: excluded.iter().map(|value| (*value).into()).collect(),
		};
		search(None, 1, vec![filter])
			.entries
			.into_iter()
			.map(|manga| manga.key)
			.collect()
	};
	let unfiltered = keys("formats", &[], &[]);
	let four_koma = keys("formats", &["4_koma"], &[]);
	assert!(!four_koma.is_empty());
	assert_ne!(four_koma, unfiltered);
	assert_eq!(four_koma, keys("genres", &["4_koma"], &[]));
	// Nearly every top title counts as a oneshot there, so excluding it shows
	let no_oneshots = keys("formats", &[], &["oneshot"]);
	assert_ne!(no_oneshots, unfiltered);
	assert_eq!(no_oneshots, keys("genres", &[], &["oneshot"]));
}

// Over 3,800 deduplicated chapters, so the list spans four pages
#[aidoku_test]
fn long_chapter_lists_arrive_whole() {
	let manga = XComic::new()
		.get_manga_update(
			Manga {
				key: "kzznn2".into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert!(manga.chapters.unwrap().len() > 3000);
}

fn chapters(key: &str) -> Vec<Chapter> {
	XComic::new()
		.get_manga_update(
			Manga {
				key: key.into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap()
		.chapters
		.unwrap()
}

#[aidoku_test]
fn merged_editions_share_one_list() {
	defaults_set("mergeEditions", DefaultValue::Bool(false));
	let single = chapters(TITLE_EDITION);
	let separate = search(Some("On the Way to Meet Mom"), 1, Vec::new())
		.entries
		.len();

	defaults_set("mergeEditions", DefaultValue::Bool(true));
	let merged = chapters(TITLE_EDITION);
	let together = search(Some("On the Way to Meet Mom"), 1, Vec::new())
		.entries
		.len();
	// "Prince Akihiko" has one team's edition in English and another in Indonesian
	defaults_set(
		"languages",
		DefaultValue::StringArray(vec!["en".into(), "id".into()]),
	);
	let bilingual = chapters("z736jp");
	defaults_set("languages", DefaultValue::Null);
	defaults_set("mergeEditions", DefaultValue::Bool(false));

	assert!(
		single
			.iter()
			.all(|chapter| chapter.language.as_deref() == Some("en"))
	);
	assert!(merged.len() > single.len());
	assert!(
		single
			.iter()
			.all(|chapter| merged.iter().any(|seen| seen.key == chapter.key))
	);
	let mut teams: Vec<&str> = merged
		.iter()
		.filter_map(|chapter| chapter.scanlators.as_ref()?.first().map(String::as_str))
		.collect();
	teams.sort_unstable();
	teams.dedup();
	assert!(teams.len() > 1);
	// Two English editions there are both by "exe scans", and two by UTOON in different case
	assert!(teams.contains(&"exe scans"));
	assert!(teams.contains(&"UTOON"));
	assert!(!teams.contains(&"UToon"));
	assert!(teams.iter().all(|team| !team.contains(" [")));
	// One copy of each numbered chapter per team, language and volume
	let mut slots: Vec<(String, Option<String>, Option<u32>, u32)> = merged
		.iter()
		.filter_map(|chapter| {
			let team = chapter.scanlators.as_ref()?.first()?.to_lowercase();
			Some((
				team,
				chapter.language.clone(),
				chapter.volume_number.map(f32::to_bits),
				chapter.chapter_number?.to_bits(),
			))
		})
		.collect();
	let numbered = slots.len();
	slots.sort_unstable();
	slots.dedup();
	assert_eq!(slots.len(), numbered);
	let numbers: Vec<f32> = merged
		.iter()
		.filter_map(|chapter| chapter.chapter_number)
		.collect();
	assert!(numbers.windows(2).all(|pair| pair[0] >= pair[1]));
	assert!(together < separate);

	let languages: Vec<&str> = bilingual
		.iter()
		.filter(|chapter| {
			chapter
				.scanlators
				.as_ref()
				.and_then(|names| names.first())
				.is_some_and(|name| name.eq_ignore_ascii_case("aakizonprincea"))
		})
		.filter_map(|chapter| chapter.language.as_deref())
		.collect();
	assert!(languages.contains(&"en"));
	assert!(languages.contains(&"id"));
}

// "Hug Me" has only a Spanish edition
#[aidoku_test]
fn title_links_open_another_language_when_needed() {
	let link = XComic::new()
		.handle_deep_link("https://xcomic.me/title/x5xjly".into())
		.unwrap()
		.unwrap();
	assert!(matches!(link, DeepLinkResult::Manga { key } if key == "5rnyl"));
}

#[aidoku_test]
fn details_and_chapters_together() {
	let manga = XComic::new()
		.get_manga_update(
			Manga {
				key: EDITION_KEY.into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.artists.is_some());
	assert!(!manga.chapters.unwrap().is_empty());
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
