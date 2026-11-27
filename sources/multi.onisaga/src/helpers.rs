use crate::{
	api::{BASE_URL, CDN_URL},
	models::{Alternative, parse_time},
};
use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Result, Viewer,
	alloc::{String, Vec, string::ToString},
	imports::html::{Document, Element},
	prelude::*,
};

pub fn status(value: &str) -> MangaStatus {
	// Search returns codes, title pages print labels.
	match value.to_lowercase().as_str() {
		"releasing" | "publishing" => MangaStatus::Ongoing,
		"completed" | "publication finished" => MangaStatus::Completed,
		"hiatus" | "publication on hiatus" => MangaStatus::Hiatus,
		"cancelled" | "publication cancelled" => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

pub fn content_rating(value: Option<&str>) -> ContentRating {
	match value.map(str::to_lowercase).as_deref() {
		Some("safe") => ContentRating::Safe,
		Some("suggestive") => ContentRating::Suggestive,
		Some("mature" | "explicit") => ContentRating::NSFW,
		_ => ContentRating::Unknown,
	}
}

pub fn viewer(vertical: bool, origin: Option<&str>) -> Viewer {
	match origin {
		_ if vertical => Viewer::Webtoon,
		Some("manhwa" | "manhua") => Viewer::Webtoon,
		Some("manga") => Viewer::RightToLeft,
		Some(_) => Viewer::LeftToRight,
		None => Viewer::Unknown,
	}
}

// Labels end in the number: "Ch 12", "Chapter 12".
pub fn chapter_number(label: &str) -> Option<f32> {
	label.rsplit(' ').next()?.parse().ok()
}

// Typed term names, a leading `-` excluding one, as the site's slugs.
pub fn terms(input: &str, include: &mut Vec<String>, exclude: &mut Vec<String>) {
	for name in input.split(',') {
		let (list, name) = match name.trim().strip_prefix('-') {
			Some(name) => (&mut *exclude, name),
			None => (&mut *include, name),
		};
		let slug = slug(name);
		if !slug.is_empty() {
			list.push(slug);
		}
	}
}

fn slug(name: &str) -> String {
	let mut slug = String::new();
	for word in name
		.to_lowercase()
		.split(|c: char| !c.is_alphanumeric())
		.filter(|word| !word.is_empty())
	{
		if !slug.is_empty() {
			slug.push('-');
		}
		slug.push_str(word);
	}
	slug
}

// Page covers ask for a thumbnail width.
fn full_size(src: String) -> String {
	match src.split_once("&w=") {
		Some((url, _)) => url.into(),
		None => src,
	}
}

pub fn title_link(element: &Element) -> Option<Manga> {
	let link = element.select_first("a[href*='/title/']")?;
	let key = link
		.attr("href")?
		.split("/title/")
		.nth(1)?
		.split(['?', '#'])
		.next()?
		.to_string();
	let title = link.text()?;
	// Trending rows have no image, but every title has a cover address.
	let cover = element
		.select_first("img[src*='/api/covers/']")
		.and_then(|img| img.attr("src"))
		.map_or_else(|| format!("{CDN_URL}/api/covers/{key}"), full_size);
	Some(Manga {
		key,
		title,
		cover: Some(cover),
		..Default::default()
	})
}

// Adult titles are a gate, filled from the API for guests who confirm their age.
pub fn gated(doc: &Document) -> Result<bool> {
	let Some(gate) = doc.select_first("main[data-ui=content-gate]") else {
		return Ok(false);
	};
	if gate.attr("data-gate").as_deref() != Some("self-confirmation") {
		bail!("OniSaga opens this only for a signed-in account allowed to see it");
	}
	Ok(true)
}

// Each chosen language the title has, with the page to read it from, or `None` for the page
// already loaded. Some languages live only in another numbering, listed after the main one. A
// matched link always has a query, since it carries `lang=`.
pub fn language_pages<'a>(
	alternatives: &[Alternative],
	languages: &'a [String],
) -> Vec<(&'a str, Option<String>)> {
	languages
		.iter()
		.filter_map(|language| {
			let mut matching = alternatives.iter().filter(|alternative| {
				alternative
					.href
					.split(['?', '&'])
					.any(|part| part.strip_prefix("lang=") == Some(language.as_str()))
			});
			let first = matching.next()?;
			let alternative = if first.current {
				first
			} else {
				matching
					.find(|alternative| alternative.current)
					.unwrap_or(first)
			};
			let url = (!alternative.current)
				.then(|| format!("{BASE_URL}{}&chapters=all", alternative.href));
			Some((language.as_str(), url))
		})
		.collect()
}

pub fn details(doc: &Document, key: &str) -> Result<Manga> {
	let hero = doc
		.select_first("section[aria-labelledby=work-title]")
		.ok_or_else(|| error!("OniSaga returned no title page"))?;
	let title = hero
		.select_first("#work-title")
		.and_then(|el| el.text())
		.ok_or_else(|| error!("OniSaga returned a title without a name"))?;
	let names = |kind: &str| {
		let names = hero
			.select("a[href*='/creator/']")
			.into_iter()
			.flatten()
			// A credit reads "Story", "Art" or "Story and art", then its names.
			.filter(|link| {
				link.parent().is_some_and(|credit| {
					let first = credit
						.select_first("a[href*='/creator/']")
						.and_then(|first| first.text());
					credit.text().zip(first).is_some_and(|(text, first)| {
						text.split(first.as_str())
							.next()
							.is_some_and(|role| role.to_lowercase().contains(kind))
					})
				})
			})
			.filter_map(|link| link.text())
			// Keep the site's order; a person credited twice appears once.
			.fold(Vec::new(), |mut names, name| {
				if !names.contains(&name) {
					names.push(name);
				}
				names
			});
		(!names.is_empty()).then_some(names)
	};
	let mut tags = hero
		.select("a[href*='genre=']")
		.into_iter()
		.flatten()
		.filter_map(|link| link.text())
		.collect::<Vec<_>>();
	tags.sort_unstable();
	tags.dedup();
	let chips = hero
		.select("[data-medium] ~ span")
		.into_iter()
		.flatten()
		.filter_map(|chip| chip.text())
		.collect::<Vec<_>>();
	let fact = |label: &str| {
		doc.select("dl dt")?
			.into_iter()
			.find(|dt| dt.text().as_deref() == Some(label))?
			.next()?
			.text()
	};
	let origin = hero
		.select_first("[data-medium]")
		.and_then(|badge| badge.attr("data-medium"));
	// Only titles above Safe carry a rating chip.
	let rating = chips
		.iter()
		.map(|chip| content_rating(Some(chip)))
		.find(|rating| *rating != ContentRating::Unknown)
		.unwrap_or(ContentRating::Safe);
	Ok(Manga {
		title,
		cover: hero
			.select_first("img[src*='/api/covers/']:not([src*='variant='])")
			.and_then(|img| img.attr("src"))
			.map(full_size),
		authors: names("story"),
		artists: names("art"),
		description: doc
			.select_first("#synopsis")
			.and_then(|el| el.untrimmed_text())
			.map(|text| text.trim().into())
			.filter(|text: &String| !text.is_empty()),
		tags: (!tags.is_empty()).then_some(tags),
		status: fact("Status").map_or(MangaStatus::Unknown, |label| status(&label)),
		content_rating: rating,
		viewer: viewer(
			fact("Layout").as_deref() == Some("Vertical scroll"),
			origin.as_deref(),
		),
		url: Some(format!("{BASE_URL}/title/{key}")),
		key: key.into(),
		..Default::default()
	})
}

// A row is one release, or a list of versions when several groups have it.
pub fn chapters(doc: &Document, slug: &str, language: &str, chapters: &mut Vec<Chapter>) {
	for row in doc.select("[data-unit]").into_iter().flatten() {
		let number = row
			.select_first("[id$='-n']")
			.and_then(|label| label.text())
			.and_then(|label| chapter_number(&label));
		// Untitled chapters only repeat their label for screen readers.
		let title = row.select_first("[id$='-t']").and_then(|span| {
			let text = span.text()?;
			let hidden = span
				.select_first(".sr-only")
				.and_then(|hidden| hidden.text());
			let title = hidden
				.and_then(|hidden| text.strip_prefix(hidden.as_str()))
				.unwrap_or(&text)
				.trim();
			(!title.is_empty()).then(|| title.into())
		});
		let versions = match row.tag_name().as_deref() {
			Some("details") => row.select("ul > li").into_iter().flatten().collect(),
			_ => Vec::from([row]),
		};
		for version in versions {
			// Rows the language lacks have no link.
			let Some(key) = version
				.attr("href")
				.or_else(|| version.select_first("a[href*='/read/']")?.attr("href"))
				.and_then(|href| Some(href.rsplit('/').next()?.into()))
			else {
				continue;
			};
			let groups = version
				.select("a[href*='/group/']")
				.into_iter()
				.flatten()
				.filter_map(|group| group.text())
				.collect::<Vec<_>>();
			chapters.push(Chapter {
				url: Some(format!("{BASE_URL}/read/{slug}/{key}")),
				key,
				title: title.clone(),
				chapter_number: number,
				date_uploaded: version
					.select_first("time[datetime]")
					.and_then(|time| time.attr("datetime"))
					.and_then(|time| parse_time(&time)),
				scanlators: (!groups.is_empty()).then_some(groups),
				language: Some(language.into()),
				..Default::default()
			});
		}
	}
}

pub fn next_page(doc: &Document) -> Option<String> {
	let href = doc
		.select_first("nav[aria-label='Chapter pages'] a[href*='cc=']")?
		.attr("href")?;
	Some(if href.starts_with('/') {
		format!("{BASE_URL}{href}")
	} else {
		href
	})
}
