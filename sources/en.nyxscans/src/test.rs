use super::*;
use serde_json as json;
type Target = Iken<NyxScans>;
const SERIES_SLUG: &str = "do-your-best-and-regret";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");
