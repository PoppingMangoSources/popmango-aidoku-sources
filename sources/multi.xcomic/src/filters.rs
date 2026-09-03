use crate::{
	XComic,
	helpers::{GENRES, LANGUAGES},
};
use aidoku::{
	BaseUrlProvider, DynamicFilters, Filter, MultiSelectFilter, Result,
	alloc::{Vec, borrow::Cow, format, vec},
	imports::net::Request,
};

/// `(id, title)` pairs in the form [`MultiSelectFilter`] takes.
type Options = Vec<(Cow<'static, str>, Cow<'static, str>)>;

fn borrowed(list: &'static [(&'static str, &'static str)]) -> Options {
	list.iter()
		.map(|(id, title)| (Cow::Borrowed(*id), Cow::Borrowed(*title)))
		.collect()
}

/// Options live in `details.group` blocks, each carrying its id in a `:`
/// attribute and its label in a `span`. Formats share the genre group.
fn fetch_genres(base_url: &str) -> Option<Options> {
	let document = Request::get(format!("{base_url}/search"))
		.ok()?
		.html()
		.ok()?;
	let group = document.select("details.group")?.find(|details| {
		details
			.select_first("summary")
			.and_then(|summary| summary.text())
			.is_some_and(|text| text.trim().to_lowercase().starts_with("genres"))
	})?;

	let mut genres: Options = Vec::new();
	for element in group.select("div")? {
		let Some(id) = element.attr(":") else {
			continue;
		};
		let id = id.trim();
		let Some(title) = element.select_first("span").and_then(|span| span.text()) else {
			continue;
		};
		let title = title.trim();
		if id.is_empty() || title.is_empty() || genres.iter().any(|(seen, _)| seen == id) {
			continue;
		}
		genres.push((Cow::Owned(id.into()), Cow::Owned(title.into())));
	}

	(!genres.is_empty()).then_some(genres)
}

fn multi_select(
	id: &'static str,
	title: &'static str,
	is_genre: bool,
	can_exclude: bool,
	options: Options,
) -> Filter {
	let (ids, options): (Vec<_>, Vec<_>) = options.into_iter().unzip();
	MultiSelectFilter {
		id: id.into(),
		title: Some(title.into()),
		is_genre,
		// Only the genre list wants the tag presentation other sources use.
		uses_tag_style: is_genre,
		can_exclude,
		options,
		ids: Some(ids),
		..Default::default()
	}
	.into()
}

impl DynamicFilters for XComic {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let genres = fetch_genres(&self.get_base_url()?).unwrap_or_else(|| borrowed(GENRES));
		// Translated language is the app's own setting; this is a separate axis,
		// and the api only takes it as includes.
		Ok(vec![
			multi_select("genres", "Genres", true, true, genres),
			multi_select(
				"original_languages",
				"Original Languages",
				false,
				false,
				borrowed(LANGUAGES),
			),
		])
	}
}
