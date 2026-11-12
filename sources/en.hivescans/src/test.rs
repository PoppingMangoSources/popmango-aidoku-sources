use super::*;
use serde_json as json;
type Target = Iken<HiveScans>;
const GENRE_EXCLUSION_SUPPORTED: bool = false;
const SORT_COUNT: usize = 5;
const SERIES_SLUG: &str = "crocodile";
const LATEST_TITLE: &str = "Latest Releases";
const FILTERS_JSON: &str = include_str!("../res/filters.json");
include!("../../../templates/iken/src/source_test.rs");

#[aidoku_test]
fn review_script_banners_are_internal_and_sized() {
	use aidoku::{HomeComponentValue, LinkValue, imports::html::Html};
	let html = Html::parse(r#"<main><section><h2>Latest Releases</h2><a href='/series/title' title='Title'><img src='https://example.com/cover.jpg'></a></section></main><script>const hvtXHeroLibrary = [{"seriesTitle":"Title","seriesUrl":"/series/title","desktopImage":"https://example.com/banner.jpg"}];</script>"#).unwrap();
	let layout = helpers::parse_home(&html, &HiveScans.params()).unwrap();
	let HomeComponentValue::ImageScroller {
		links,
		width,
		height,
		..
	} = &layout.components[0].value
	else {
		panic!("Expected banners");
	};
	assert_eq!((*width, *height), (Some(400), Some(150)));
	assert!(matches!(&links[0].value,Some(LinkValue::Manga(m)) if m.key == "/series/title"));
	assert!(links[0].subtitle.is_none());
}
