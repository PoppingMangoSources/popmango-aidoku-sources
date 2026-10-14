use crate::{
	helpers::{key_from_query, key_from_url, split_label},
	models::{Series, chapter_list},
};
use aidoku::alloc::{String, Vec, vec};
use aidoku_test::aidoku_test;

fn chapters(json: &str) -> Vec<aidoku::Chapter> {
	chapter_list(serde_json::from_str(json).expect("chapter entries"), false)
}

fn keys(list: &[aidoku::Chapter]) -> Vec<&str> {
	list.iter().map(|chapter| chapter.key.as_str()).collect()
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
		r#"[{"id": 7, "label": "Chapter 2: Rain", "at": 1700000000000, "n": 2, "vol": "3",
			"group": {"name": "Alpha"}}]"#,
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
fn test_volume_either_way_round() {
	assert_eq!(chapters(r#"[{"id": 1, "n": 1}]"#)[0].volume_number, None);
	assert_eq!(
		chapters(r#"[{"id": 1, "n": 1, "vol": 2}]"#)[0].volume_number,
		Some(2.0)
	);
	assert_eq!(
		chapters(r#"[{"id": 1, "n": 1, "vol": "2"}]"#)[0].volume_number,
		Some(2.0)
	);
}

#[aidoku_test]
fn test_title_naming_the_group_is_dropped() {
	let list = chapters(r#"[{"id": 1, "n": 1, "label": "Alpha", "group": {"name": "alpha"}}]"#);
	assert_eq!(list[0].title, None);
}

#[aidoku_test]
fn test_alternates_keep_one_per_group() {
	let list = chapters(
		r#"[{"id": 1, "n": 9, "group": {"name": "Alpha"}, "alts": [
			{"id": 2, "group": {"name": "alpha"}},
			{"id": 3, "group": {"name": "Beta"}},
			{"id": 4, "label": "Chapter 9 [Beta]"}
		]}]"#,
	);
	assert_eq!(keys(&list), ["1", "3"]);
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
	let list = chapters(
		r#"[
			{"id": 1, "n": 1, "group": {"name": "Official"}, "alts": [{"id": 2, "group": {"name": "Rolia"}}]},
			{"id": 3, "n": 2, "group": {"name": "Rolia"}, "alts": [{"id": 1, "group": {"name": "Official"}}]}
		]"#,
	);
	assert_eq!(keys(&list), ["3", "1", "2"]);
}

// 20th Century Boys, Vol. 1 Ch. 1: the site lists one row for each of these four groups
#[aidoku_test]
fn test_group_keeps_its_best_upload() {
	let list = chapters(
		r#"[
			{"id": 90074, "n": 1, "vol": 1, "label": "Chapter 1", "w": 1817, "mb": 8.2,
				"group": {"name": "Band of the Hawks"}},
			{"id": 2351311, "n": 1, "vol": 1, "label": "Friends", "w": 3790, "mb": 238.6,
				"group": {"name": "Official"}},
			{"id": 1491694, "n": 1, "vol": 1, "label": "Chapter 1", "w": 1200, "mb": 19.1,
				"group": {"name": "WeebCentral"}, "alts": [
				{"id": 2168860, "label": "Friends", "w": 3790, "mb": 238.6, "group": {"name": "Official"}},
				{"id": 4176441, "label": "Chapter 1", "w": 720, "mb": 9.5, "group": {"name": "Rolia Scans"}},
				{"id": 1268949, "label": "Chapter 1", "w": 1200, "mb": 19.1, "group": {"name": "Rolia Scans"}}
			]}
		]"#,
	);
	assert_eq!(keys(&list), ["90074", "2351311", "1491694", "1268949"]);
	assert_eq!(list[1].title.as_deref(), Some("Friends"));
}

#[aidoku_test]
fn test_volumes_order_the_list() {
	let list = chapters(
		r#"[
			{"id": 1, "n": 30},
			{"id": 2, "n": 5, "vol": 2},
			{"id": 3, "n": 2, "vol": 1},
			{"id": 4, "n": 1, "vol": 3, "group": {"name": "Official"}},
			{"id": 5, "n": 1, "vol": 1, "group": {"name": "Official"}},
			{"id": 6, "vol": 2, "label": "Vol. 2", "group": {"name": "Official"}}
		]"#,
	);
	assert_eq!(keys(&list), ["1", "4", "6", "2", "3", "5"]);
	assert_eq!(list[2].title, None);
}

#[aidoku_test]
fn test_main_only_skips_alternates() {
	let list = chapter_list(
		serde_json::from_str(
			r#"[
				{"id": 2351311, "n": 1, "vol": 1, "group": {"name": "Official"}},
				{"id": 1491694, "n": 1, "vol": 1, "group": {"name": "WeebCentral"}, "alts": [
					{"id": 2168860, "group": {"name": "Official"}},
					{"id": 1268949, "group": {"name": "Rolia Scans"}}
				]}
			]"#,
		)
		.expect("chapter entries"),
		true,
	);
	assert_eq!(keys(&list), ["2351311", "1491694"]);
}

#[aidoku_test]
fn test_tag_groups_follow_the_site_picker() {
	use crate::filters::group_index;
	assert_eq!(group_index("Romance", None), 0);
	assert_eq!(group_index("Harem", None), 1);
	assert_eq!(group_index("Office", Some("Setting-Scene")), 1);
	assert_eq!(group_index("Smut", None), 2);
	assert_eq!(group_index("Doujinshi", None), 3);
}
