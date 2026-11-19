use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

pub fn content_rating() -> String {
	defaults_get::<String>("contentRating").unwrap_or_else(|| "erotica".into())
}

// The site reads no types as every type; an unknown one matches nothing.
pub fn types() -> Vec<String> {
	match defaults_get::<Vec<String>>("types") {
		Some(types) if types.is_empty() => vec!["none".into()],
		Some(types) if types.len() < 5 => types,
		_ => Vec::new(),
	}
}

pub fn excluded_genres() -> Vec<String> {
	defaults_get::<Vec<String>>("excludedGenres").unwrap_or_default()
}
