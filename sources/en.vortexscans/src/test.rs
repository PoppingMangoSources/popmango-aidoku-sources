use super::*;
use serde_json as json;
type Target = Iken<VortexScans>;
const SERIES_SLUG: &str = "the-crazy-villain-regains-his-sanity";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");
