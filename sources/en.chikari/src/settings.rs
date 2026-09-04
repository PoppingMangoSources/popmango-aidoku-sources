use aidoku::{
	alloc::{String, Vec},
	imports::defaults::defaults_get,
};

const CONTENT_TYPES_KEY: &str = "contentTypes";
const CONTENT_RATINGS_KEY: &str = "contentRatings";

/// An unset list means the declared default; one the reader emptied stays empty,
/// so clearing every box cannot silently re-enable everything.
fn stored_list(key: &str, fallback: &[&str]) -> Vec<String> {
	defaults_get::<Vec<String>>(key)
		.unwrap_or_else(|| fallback.iter().copied().map(Into::into).collect())
}

pub fn content_types() -> Vec<String> {
	stored_list(CONTENT_TYPES_KEY, &["manga", "manhwa", "manhua"])
}

pub fn content_ratings() -> Vec<String> {
	stored_list(CONTENT_RATINGS_KEY, &["safe", "suggestive"])
}

pub fn adult() -> bool {
	content_ratings()
		.iter()
		.any(|r| r == "erotica" || r == "pornographic")
}
