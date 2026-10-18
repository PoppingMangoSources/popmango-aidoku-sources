use aidoku::{
	alloc::{String, Vec, vec},
	imports::defaults::defaults_get,
};

pub const DEFAULT_BASE_URL: &str = "https://kagane.cx";

const SOURCE_TYPES: &[&str] = &["Official", "Unofficial", "Mixed"];

fn flag(key: &str) -> bool {
	defaults_get::<bool>(key).unwrap_or_default()
}

fn list(key: &str) -> Vec<String> {
	defaults_get::<Vec<String>>(key).unwrap_or_default()
}

pub fn base_url() -> String {
	defaults_get::<String>("url")
		.filter(|url| !url.is_empty())
		.unwrap_or_else(|| DEFAULT_BASE_URL.into())
}

/// The app's languages as the API spells them, or English when none is picked.
pub fn languages() -> Vec<String> {
	defaults_get::<Vec<String>>("languages")
		.filter(|languages| !languages.is_empty())
		.map(|languages| {
			languages
				.into_iter()
				.map(|language| match language.as_str() {
					"pt-BR" => "pt-br".into(),
					_ => language,
				})
				.collect()
		})
		.unwrap_or_else(|| vec!["en".into()])
}

pub fn age_ratings() -> Vec<String> {
	defaults_get::<Vec<String>>("ageRatings")
		.unwrap_or_else(|| vec!["Safe".into(), "FifteenPlus".into()])
}

pub fn show_pornographic() -> bool {
	flag("showPornographic")
}

/// The source types to ask for, or none when every one is on, since nothing is then narrowed.
pub fn source_types() -> Option<Vec<String>> {
	match defaults_get::<Vec<String>>("sourceType") {
		Some(types) if types.len() < SOURCE_TYPES.len() => Some(types),
		_ => None,
	}
}

pub fn hidden_genres() -> Vec<String> {
	list("hiddenGenres")
}

pub fn hidden_tags() -> Vec<String> {
	list("hiddenTags")
}

pub fn show_source() -> bool {
	flag("showSource")
}

pub fn show_edition() -> bool {
	flag("showEdition")
}

pub fn show_spoiler_tags() -> bool {
	flag("showSpoilerTags")
}

pub fn data_saver() -> bool {
	flag("dataSaver")
}

pub fn popular_period() -> String {
	defaults_get::<String>("popularPeriod").unwrap_or_else(|| "week".into())
}
