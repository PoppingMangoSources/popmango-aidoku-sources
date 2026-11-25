use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

pub fn languages() -> Vec<String> {
	defaults_get::<Vec<String>>("languages").unwrap_or_else(|| vec!["en".into()])
}

pub fn content_rating() -> String {
	defaults_get::<String>("contentRating").unwrap_or_else(|| "safe".into())
}

pub fn types() -> Option<Vec<String>> {
	defaults_get::<Vec<String>>("types").filter(|types| types.len() < 16)
}

pub fn excluded_genres() -> Vec<String> {
	defaults_get::<Vec<String>>("excludedGenres").unwrap_or_default()
}

fn names(key: &str) -> Vec<String> {
	defaults_get::<Vec<String>>(key)
		.unwrap_or_default()
		.into_iter()
		.map(|name| name.trim().to_lowercase())
		.filter(|name| !name.is_empty())
		.collect()
}

pub fn excluded_tags() -> Vec<String> {
	names("excludedTags")
}

pub fn deduplicate_chapters() -> String {
	defaults_get::<String>("deduplicateChapters").unwrap_or_else(|| "off".into())
}

pub fn preferred_uploaders() -> Vec<String> {
	names("preferredUploaders")
}

pub fn blocked_uploaders() -> Vec<String> {
	names("blockedUploaders")
}
