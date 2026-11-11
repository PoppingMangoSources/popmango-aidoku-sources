use super::*;
use serde_json as json;
type Target = Iken<EternalMangas>;
const SERIES_SLUG: &str = "la-doncella-secreta-del-conde";
const LATEST_TITLE: &str = "Últimas Actualizaciones";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");
