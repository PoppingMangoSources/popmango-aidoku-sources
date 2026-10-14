use crate::{
	helpers::{key_from_query, key_from_url, split_label, split_range},
	models::{ChapterEntry, Series, chapter_list},
};
use aidoku::alloc::{String, Vec, vec};
use aidoku_test::aidoku_test;

fn chapters(json: &str) -> Vec<aidoku::Chapter> {
	serde_json::from_str::<ChapterEntry>(json)
		.expect("chapter entry")
		.into_chapters()
}

#[aidoku_test]
fn test_label_titles() {
	let title = |label: &str, number: f64| split_label(label, Some(number)).0;
	assert_eq!(
		title("Chapter 12: The Return", 12.0),
		Some(String::from("The Return"))
	);
	assert_eq!(
		title("Ch. 12 - The Return", 12.0),
		Some("The Return".into())
	);
	assert_eq!(title("Chapter 12", 12.0), None);
	assert_eq!(title("12", 12.0), None);
	assert_eq!(title("12: Chapter 12", 12.0), None);
	// a title that merely contains digits is still a title
	assert_eq!(title("Chapter 3: 1984", 3.0), Some("1984".into()));
	// a oneshot is numbered 0, and its label is the only name it has
	assert_eq!(title("Oneshot", 0.0), Some("Oneshot".into()));
	assert_eq!(title("Chapter 5: A &amp; B", 5.0), Some("A & B".into()));
}

#[aidoku_test]
fn test_label_groups() {
	let (title, group) = split_label("Chapter 4: Night [Hive Scans]", Some(4.0));
	assert_eq!(title, Some("Night".into()));
	assert_eq!(group, Some("Hive Scans".into()));
	assert_eq!(split_label("Chapter 4 []", Some(4.0)).1, None);
}

#[aidoku_test]
fn test_chapter_from_entry() {
	let list = chapters(
		r#"{"id": 7, "label": "Chapter 2: Rain", "at": 1700000000000, "n": 2, "vol": "3",
			"group": {"name": "Alpha"}}"#,
	);
	assert_eq!(list.len(), 1);
	let chapter = &list[0];
	assert_eq!(chapter.key, "7");
	assert_eq!(chapter.title.as_deref(), Some("Rain"));
	assert_eq!(chapter.chapter_number, Some(2.0));
	assert_eq!(chapter.volume_number, Some(3.0));
	assert_eq!(chapter.date_uploaded, Some(1700000000));
	assert_eq!(chapter.scanlators, Some(vec!["Alpha".into()]));
	assert_eq!(
		chapter.url.as_deref(),
		Some("https://toonkotsu.com/chapter/7")
	);
}

#[aidoku_test]
fn test_volume_zero_is_none() {
	assert_eq!(
		chapters(r#"{"id": 1, "n": 1, "vol": 0}"#)[0].volume_number,
		None
	);
	assert_eq!(
		chapters(r#"{"id": 1, "n": 1, "vol": "0"}"#)[0].volume_number,
		None
	);
	assert_eq!(chapters(r#"{"id": 1, "n": 1}"#)[0].volume_number, None);
	assert_eq!(
		chapters(r#"{"id": 1, "n": 1, "vol": 2}"#)[0].volume_number,
		Some(2.0)
	);
}

#[aidoku_test]
fn test_title_naming_the_group_is_dropped() {
	let list = chapters(r#"{"id": 1, "n": 1, "label": "Alpha", "group": {"name": "alpha"}}"#);
	assert_eq!(list[0].title, None);
}

#[aidoku_test]
fn test_alternates_keep_one_per_group() {
	let list = chapters(
		r#"{"id": 1, "n": 9, "group": {"name": "Alpha"}, "alts": [
			{"id": 2, "group": {"name": "alpha"}},
			{"id": 3, "group": {"name": "Beta"}},
			{"id": 4, "label": "Chapter 9 [Beta]"}
		]}"#,
	);
	let keys: Vec<&str> = list.iter().map(|chapter| chapter.key.as_str()).collect();
	assert_eq!(keys, ["1", "3"]);
	assert!(
		list.iter()
			.all(|chapter| chapter.chapter_number == Some(9.0))
	);
}

#[aidoku_test]
fn test_card_is_compact() {
	let series: Series = serde_json::from_str(
		r#"{"hid": "a1b2c", "slug": "some-title", "title": "Some Title", "cover": "/c/a1b2c/220.webp",
			"description": "Long text", "tags": ["Action"], "authors": ["Someone"]}"#,
	)
	.expect("series");
	let manga = series.into_manga().expect("card");
	assert_eq!(manga.key, "a1b2c");
	assert_eq!(
		manga.url.as_deref(),
		Some("https://toonkotsu.com/title/a1b2c-some-title")
	);
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://toonkotsu.com/c/a1b2c/460.webp")
	);
	assert!(manga.description.is_none() && manga.tags.is_none() && manga.authors.is_none());
}

#[aidoku_test]
fn test_series_without_key_is_skipped() {
	let series: Series = serde_json::from_str(r#"{"title": "No Key"}"#).expect("series");
	assert!(series.into_manga().is_none());
}

#[aidoku_test]
fn test_links() {
	assert_eq!(
		key_from_url("https://toonkotsu.com/title/a1b2c-some-title"),
		Some("a1b2c".into())
	);
	assert_eq!(
		key_from_url("https://toonkotsu.com/title/a1b2c"),
		Some("a1b2c".into())
	);
	assert_eq!(key_from_url("https://toonkotsu.com/title/Abcde"), None);
	assert_eq!(key_from_url("https://toonkotsu.com/chapter/123"), None);
	assert_eq!(key_from_query("solo leveling"), None);
}

#[aidoku_test]
fn test_ranges() {
	assert_eq!(split_range("2018-2022"), ("2018".into(), "2022".into()));
	assert_eq!(split_range(" 2020 "), ("2020".into(), "2020".into()));
	assert_eq!(split_range("20-"), ("20".into(), String::new()));
	assert_eq!(split_range("abc"), (String::new(), String::new()));
}

// ciphertext from `openssl enc -aes-128-ctr` with the nonce's eight bytes, then a zero counter
#[aidoku_test]
fn test_page_decryption() {
	let mut data = [
		143u8, 71, 251, 242, 85, 194, 201, 19, 18, 43, 140, 142, 74, 5, 10, 239, 3, 148, 93, 233,
		36, 68, 199, 150, 121, 138, 32, 152, 85, 244, 217, 5, 205, 65, 73, 28, 253, 84, 39, 239,
	];
	crate::reader::decrypt_page(&mut data, "AAECAwQFBgcICQoLDA0ODw", "oKGio6Slpqc")
		.expect("decrypt");
	assert_eq!(&data[..], b"Toonkotsu pages cross a block boundary!!");
}

#[aidoku_test]
fn test_release_listed_twice_is_one_chapter() {
	let entries: Vec<ChapterEntry> = serde_json::from_str(
		r#"[
			{"id": 1, "n": 1, "group": {"name": "Official"}, "alts": [{"id": 2, "group": {"name": "Rolia"}}]},
			{"id": 3, "n": 2, "group": {"name": "Rolia"}, "alts": [{"id": 1, "group": {"name": "Official"}}]}
		]"#,
	)
	.expect("entries");
	let keys: Vec<String> = chapter_list(entries)
		.into_iter()
		.map(|chapter| chapter.key)
		.collect();
	assert_eq!(keys, ["1", "2", "3"]);
}
