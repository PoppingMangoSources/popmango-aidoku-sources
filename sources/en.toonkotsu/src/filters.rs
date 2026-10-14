use crate::{
	Toonkotsu,
	api::{self, BASE_URL, json_request},
	helpers::is_unlisted_tag,
	models::{Facet, Facets},
	settings,
};
use aidoku::{
	DynamicFilters, DynamicSettings, Filter, GroupSetting, MultiSelectFilter, MultiSelectSetting,
	Result, Setting,
	alloc::{String, Vec, format, vec},
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

/// The site only offers these once erotica is allowed.
const EROTIC_TAGS: &[&str] = &["Hentai", "Adult", "Smut"];

pub fn group_ids() -> impl Iterator<Item = &'static str> {
	GROUPS.iter().map(|(id, ..)| *id)
}

/// Tags the site offers for browsing, fetched once per filter sheet or settings page.
fn listed_facets() -> Result<Vec<Facet>> {
	let facets: Facets =
		api::parse(json_request(format!("{BASE_URL}/cdn/v1/facets?sfw=0"))?.send()?)?;
	Ok(facets
		.genres
		.unwrap_or_default()
		.into_iter()
		.filter(|facet| {
			!is_unlisted_tag(&facet.name) && facet.category.as_deref() != Some("Demographic")
		})
		.collect())
}

impl DynamicFilters for Toonkotsu {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let ratings = settings::content_ratings();
		let allows = |rating: &str| ratings.iter().any(|value| value == rating);
		let mut grouped: Vec<Vec<String>> = vec![Vec::new(); GROUPS.len()];

		for facet in listed_facets()? {
			// The catalogue flags some tags adult, which the site only offers with that rating.
			if facet.is_adult == Some(true) && !allows("pornographic") {
				continue;
			}
			if EROTIC_TAGS.contains(&facet.name.as_str())
				&& !allows("erotica")
				&& !allows("pornographic")
			{
				continue;
			}
			let category = facet.category.unwrap_or_default();
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

/// The tag list lives on the site, so the hidden tags are offered from it rather than
/// kept in `settings.json`.
impl DynamicSettings for Toonkotsu {
	fn get_dynamic_settings(&self) -> Result<Vec<Setting>> {
		let mut tags: Vec<String> = listed_facets()?
			.into_iter()
			.map(|facet| facet.name)
			.collect();
		tags.sort();
		Ok(vec![
			GroupSetting {
				title: "Exclusions".into(),
				footer: Some(
					"Titles carrying any of these are left out of Discover, listings and search, \
					 unless a search asks for the tag."
						.into(),
				),
				items: vec![
					MultiSelectSetting {
						key: "hiddenTags".into(),
						title: "Hidden Tags".into(),
						values: tags.into_iter().map(Into::into).collect(),
						refreshes: Some(vec!["content".into()]),
						..Default::default()
					}
					.into(),
				],
				..Default::default()
			}
			.into(),
		])
	}
}
