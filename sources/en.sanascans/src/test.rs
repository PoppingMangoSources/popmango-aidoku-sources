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
