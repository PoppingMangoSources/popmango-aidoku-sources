use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

/// Pornographic titles are opt-in, as they are on the site.
const DEFAULT_RATINGS: &[&str] = &["safe", "suggestive", "erotica"];

/// The options under Original Languages in settings.json.
const LANGUAGE_COUNT: usize = 9;

fn stored_list(key: &str, default: &[&str]) -> Vec<String> {
	defaults_get::<Vec<String>>(key)
		.unwrap_or_else(|| default.iter().map(|value| (*value).into()).collect())
}

pub fn content_ratings() -> Vec<String> {
	stored_list("contentRatings", DEFAULT_RATINGS)
}

/// The original languages to browse, or none when every one is on, since the catalogue then
/// has nothing to narrow.
pub fn original_languages() -> Vec<String> {
	match defaults_get::<Vec<String>>("originalLanguages") {
		// The catalogue matches nothing for a language it doesn't know, which is what
		// clearing every language asks for.
		Some(languages) if languages.is_empty() => vec!["none".into()],
		Some(languages) if languages.len() < LANGUAGE_COUNT => languages,
		_ => Vec::new(),
	}
}

pub fn hidden_tags() -> Vec<String> {
	defaults_get::<Vec<String>>("hiddenTags").unwrap_or_default()
}

pub fn deduplicate_chapters() -> bool {
	defaults_get::<bool>("deduplicateChapters").unwrap_or_default()
}

pub fn show_upload_size() -> bool {
	defaults_get::<bool>("showUploadSize").unwrap_or_default()
}

/// The home sections to load, or every one when the setting was never stored.
pub fn home_sections() -> Option<Vec<String>> {
	defaults_get::<Vec<String>>("homeSections")
}
