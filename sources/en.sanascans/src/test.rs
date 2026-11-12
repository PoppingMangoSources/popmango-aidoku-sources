use super::*;
use serde_json as json;
type Target = Iken<SanaScans>;
const GENRE_EXCLUSION_SUPPORTED: bool = true;
const SORT_COUNT: usize = 5;
const SERIES_SLUG: &str =
	"after-15-years-petrified-the-former-princess-is-adored-by-her-husband-and-daughter";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");

#[aidoku_test::aidoku_test]
fn live_advanced_filters_and_genre_exclusions() {
	use aidoku::{FilterValue, Manga, Source, alloc::vec};
	let source = Target::new();
	let impossible = source
		.get_search_manga_list(
			None,
			1,
			vec![
				FilterValue::Text {
					id: "minChapters".into(),
					value: "100000".into(),
				},
				FilterValue::Text {
					id: "createdAfter".into(),
					value: "2099-01-01".into(),
				},
			],
		)
		.unwrap();
	assert!(impossible.entries.is_empty());
	let results = source
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genreIds".into(),
				included: vec!["1".into()],
				excluded: vec!["2".into()],
			}],
		)
		.unwrap();
	assert!(!results.entries.is_empty());
	let filters: json::Value = json::from_str(FILTERS_JSON).unwrap();
	let genre = filters
		.as_array()
		.unwrap()
		.iter()
		.find(|filter| filter["id"] == "genreIds")
		.unwrap();
	let position = genre["ids"]
		.as_array()
		.unwrap()
		.iter()
		.position(|id| id == "2")
		.unwrap();
	let excluded = genre["options"][position].as_str().unwrap().trim();
	let key = results.entries.into_iter().next().unwrap().key;
	let details = source
		.get_manga_update(
			Manga {
				key,
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert!(
		details
			.tags
			.unwrap_or_default()
			.iter()
			.all(|tag| tag.trim() != excluded)
	);
}
