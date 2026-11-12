use super::*;
use serde_json as json;
type Target = Iken<HijalaScans>;
const GENRE_EXCLUSION_SUPPORTED: bool = true;
const SORT_COUNT: usize = 5;
const SERIES_SLUG: &str = "blood-rain-3";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");
