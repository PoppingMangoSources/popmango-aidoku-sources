use super::*;
use serde_json as json;
type Target = Iken<MagusManga>;
const SERIES_SLUG: &str = "turning-the-mad-dog-into-a-genteel-lord";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");
