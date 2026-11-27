use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

pub fn languages() -> Vec<String> {
	defaults_get::<Vec<String>>("languages").unwrap_or_else(|| vec!["en".into()])
}

pub fn content_ratings() -> Vec<String> {
	defaults_get::<Vec<String>>("contentRatings")
		.unwrap_or_else(|| vec!["safe".into(), "suggestive".into()])
}

// None while every origin is chosen, so titles without one still show.
pub fn origins() -> Option<Vec<String>> {
	defaults_get::<Vec<String>>("origins").filter(|origins| origins.len() < 7)
}

pub fn excluded_genres() -> Vec<String> {
	defaults_get::<Vec<String>>("excludedGenres").unwrap_or_default()
}

pub fn deduplicate_chapters() -> bool {
	defaults_get::<bool>("deduplicateChapters").unwrap_or(false)
}
