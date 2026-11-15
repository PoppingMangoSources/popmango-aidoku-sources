use super::*;
use serde_json as json;
type Target = Iken<VortexScans>;
const GENRE_EXCLUSION_SUPPORTED: bool = false;
const SORT_COUNT: usize = 5;
const SERIES_SLUG: &str = "the-crazy-villain-regains-his-sanity";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");

// The site's slider mixes cover slides into its banners and renders most slides lazily
#[aidoku_test]
fn home_banner_row_holds_only_banner_art() {
	let layout = Target::new().get_home().unwrap();
	let aidoku::HomeComponentValue::ImageScroller { links, .. } = &layout.components[0].value
	else {
		panic!("Expected the banner row first");
	};
	let images: Vec<_> = links.iter().map(|link| link.image_url.as_deref()).collect();
	assert!(links.len() >= 3, "{images:?}");
	assert!(
		images.iter().all(|url| url
			.is_some_and(|url| url.contains("/upload/series/banner/") && url.contains("w=680"))),
		"{images:?}"
	);
}
