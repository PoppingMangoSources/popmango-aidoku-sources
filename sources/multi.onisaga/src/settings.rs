use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

pub const ORIGINS: [&str; 7] = [
	"manga",
	"manhwa",
	"manhua",
	"comic",
	"oel",
	"other_asian",
	"other",
];

pub fn languages() -> Vec<String> {
	defaults_get::<Vec<String>>("languages").unwrap_or_else(|| vec!["en".into()])
}

pub fn content_ratings() -> Vec<String> {
	defaults_get::<Vec<String>>("contentRatings").unwrap_or_else(|| vec!["safe".into()])
}

// None while every origin is chosen, so titles without one still show.
pub fn origins() -> Option<Vec<String>> {
	defaults_get::<Vec<String>>("origins").filter(|origins| origins.len() < ORIGINS.len())
}

pub fn excluded_genres() -> Vec<String> {
	defaults_get::<Vec<String>>("excludedGenres").unwrap_or_default()
}
