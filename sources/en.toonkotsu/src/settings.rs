use aidoku::{
	alloc::{String, Vec},
	imports::defaults::defaults_get,
};

/// Pornographic titles are opt-in, as they are on the site.
const DEFAULT_RATINGS: &[&str] = &["safe", "suggestive", "erotica"];

/// The catalogue files a series by its original language, so a type filters as the
/// languages it is written in.
pub const TYPES: &[(&str, &[&str])] = &[
	("manga", &["ja", "ja-ro"]),
	("manhwa", &["ko", "ko-ro"]),
	("manhua", &["zh", "zh-hk", "zh-ro"]),
	("oel", &["en"]),
	("other", &["xx"]),
];

fn stored_list(key: &str, default: &[&str]) -> Vec<String> {
	defaults_get::<Vec<String>>(key)
		.unwrap_or_else(|| default.iter().map(|value| (*value).into()).collect())
}

pub fn content_ratings() -> Vec<String> {
	stored_list("contentRatings", DEFAULT_RATINGS)
}

pub fn types() -> Vec<String> {
	stored_list(
		"types",
		&TYPES.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
	)
}

/// Original languages the selected types are written in, or none when every type is on,
/// since the catalogue then has nothing to narrow.
pub fn original_languages() -> Vec<String> {
	let selected = types();
	if selected.len() >= TYPES.len() {
		return Vec::new();
	}
	TYPES
		.iter()
		.filter(|(id, _)| selected.iter().any(|value| value == id))
		.flat_map(|(_, languages)| languages.iter().map(|language| (*language).into()))
		.collect()
}

pub fn hidden_tags() -> Vec<String> {
	defaults_get::<Vec<String>>("hiddenTags").unwrap_or_default()
}
