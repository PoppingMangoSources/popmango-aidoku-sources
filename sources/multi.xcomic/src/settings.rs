use aidoku::{
	Result,
	alloc::{String, Vec},
	imports::defaults::defaults_get,
	prelude::*,
};

const LANGUAGES_KEY: &str = "languages";

/// Selected languages, in the underscore form the API expects.
pub fn get_languages() -> Result<Vec<String>> {
	defaults_get::<Vec<String>>(LANGUAGES_KEY)
		.map(|languages| {
			languages
				.into_iter()
				.map(|language| match language.as_str() {
					"pt-BR" => "pt_br".into(),
					"es-419" => "es_419".into(),
					_ => language,
				})
				.collect()
		})
		.ok_or(error!("Unable to fetch languages"))
}

/// Inverse of [`get_languages`]: API code back to the BCP 47 form Aidoku uses.
pub fn normalize_language(language: &str) -> Option<String> {
	let language = language.trim();
	if language.is_empty() {
		return None;
	}
	Some(match language {
		"pt_br" => "pt-BR".into(),
		"es_419" => "es-419".into(),
		_ => language.replace('_', "-"),
	})
}
