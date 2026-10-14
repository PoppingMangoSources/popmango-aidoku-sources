use crate::{
	Toonkotsu,
	api::{self, facets_request},
	helpers::is_unlisted_tag,
	models::Facets,
	settings,
};
use aidoku::{
	DynamicFilters, Filter, MultiSelectFilter, Result,
	alloc::{String, Vec, vec},
};

/// The catalogue files each tag under a category, and leaves the main genres without one.
/// Every group filters the same parameter, so a reader can mix them freely.
const GROUPS: &[(&str, &str, Option<&str>)] = &[
	("genres", "Genres", None),
	("themes", "Themes", Some("Theme")),
	("cast", "Cast", Some("Cast")),
	("setting", "Setting", Some("Setting")),
	("format", "Format", Some("Technical")),
];

pub fn group_ids() -> impl Iterator<Item = &'static str> {
	GROUPS.iter().map(|(id, ..)| *id)
}

impl DynamicFilters for Toonkotsu {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let facets: Facets = api::parse(facets_request()?.send()?)?;
		let ratings = settings::content_ratings();
		let allows = |rating: &str| ratings.iter().any(|value| value == rating);
		let mut grouped: Vec<Vec<String>> = vec![Vec::new(); GROUPS.len()];

		for facet in facets.genres.unwrap_or_default() {
			let category = facet.category.unwrap_or_default();
			if is_unlisted_tag(&facet.name) || category == "Demographic" {
				continue;
			}
			// The site only offers an adult tag once the matching rating is on.
			if facet.is_adult == Some(true) && !allows("pornographic") {
				continue;
			}
			let index = GROUPS
				.iter()
				.position(|(_, _, prefix)| {
					prefix.is_some_and(|prefix| category.starts_with(prefix))
				})
				.unwrap_or(if category.is_empty() { 0 } else { 1 });
			grouped[index].push(facet.name);
		}

		Ok(GROUPS
			.iter()
			.zip(grouped)
			.filter(|(_, options)| !options.is_empty())
			.map(|((id, title, _), mut options)| {
				options.sort();
				MultiSelectFilter {
					id: (*id).into(),
					title: Some((*title).into()),
					is_genre: true,
					uses_tag_style: true,
					can_exclude: true,
					options: options.into_iter().map(Into::into).collect(),
					..Default::default()
				}
				.into()
			})
			.collect())
	}
}
