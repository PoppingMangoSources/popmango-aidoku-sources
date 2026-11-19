use crate::{BASE_URL, settings};
use aidoku::{
	ContentRating, Manga, MangaPageResult, MangaStatus, Result, Viewer,
	alloc::{String, Vec, format, string::ToString},
	helpers::{
		string::PlainText,
		uri::{QueryParameters, encode_uri_component},
	},
	imports::{
		html::{Document, Element, ElementList},
		net::Request,
	},
	prelude::*,
};

// Series sit under their type; `/comic/` redirects to it.
const TYPES: [&str; 6] = ["manhwa", "manga", "manhua", "webtoon", "comic", "western"];

// Type pages that share the series path.
const TYPE_PAGES: [&str; 5] = [
	"browse",
	"top-rated",
	"most-popular",
	"trending",
	"most-favorited",
];

// Sorts in `filters.json` order.
pub const SORTS: [&str; 4] = ["latest", "popular", "rating", "a-z"];

// The site's content setting, a cookie every page reads.
pub struct Content {
	pub rating: String,
	pub types: Vec<String>,
	pub blocked: Vec<String>,
}

impl Content {
	pub fn new() -> Self {
		Self {
			rating: settings::content_rating(),
			types: settings::types(),
			blocked: settings::excluded_genres(),
		}
	}

	pub fn request(&self, url: &str) -> Result<Request> {
		let list = |values: &[String]| {
			values
				.iter()
				.map(|value| format!("\"{value}\""))
				.collect::<Vec<_>>()
				.join(",")
		};
		let prefs = format!(
			r#"{{"rating":"{}","types":[{}],"blockedGenres":[{}]}}"#,
			self.rating,
			list(&self.types),
			list(&self.blocked)
		);
		Ok(Request::get(url)?.header(
			"Cookie",
			&format!("reader_prefs={}", encode_uri_component(prefs)),
		))
	}
}

pub struct SearchParams {
	pub query: Option<String>,
	pub sort: &'static str,
	pub status: String,
	pub author: Option<String>,
	pub genres: Vec<String>,
	pub excluded_genres: Vec<String>,
	pub content: Content,
}

impl SearchParams {
	pub fn new(query: Option<String>) -> Self {
		Self {
			query,
			sort: SORTS[0],
			status: String::new(),
			author: None,
			genres: Vec::new(),
			excluded_genres: Vec::new(),
			content: Content::new(),
		}
	}

	pub fn sorted(sort: &'static str) -> Self {
		Self {
			sort,
			..Self::new(None)
		}
	}

	pub fn page(&self, page: i32) -> Result<MangaPageResult> {
		let mut query = QueryParameters::new();
		if self.query.is_some() {
			query.push("search", self.query.as_deref());
		}
		query.push_encoded("sort", Some(self.sort));
		if !self.status.is_empty() {
			query.push_encoded("status", Some(&self.status));
		}
		if let Some(author) = &self.author {
			query.push("author", Some(&slugify(author)));
		}
		if !self.genres.is_empty() {
			query.push_encoded("genres", Some(&self.genres.join(",")));
		}
		if !self.excluded_genres.is_empty() {
			query.push_encoded("excludeGenres", Some(&self.excluded_genres.join(",")));
		}
		query.push_encoded("page", Some(&page.to_string()));
		let document = self
			.content
			.request(&format!("{BASE_URL}/comics?{query}"))?
			.html()?;
		Ok(MangaPageResult {
			entries: cards(document.select("a[href]")),
			has_next_page: document
				.select_first("a[aria-label='Go to next page'][href]")
				.is_some(),
		})
	}
}

// Series keys are slugs, which `/comic/` resolves for series and chapters alike.
pub fn series_path(path: &str) -> Option<(String, Option<String>)> {
	let mut parts = path.trim_matches('/').split(['/', '?', '#']);
	let kind = parts.next()?;
	let slug = parts.next().filter(|slug| !slug.is_empty())?;
	if !TYPES.contains(&kind) || TYPE_PAGES.contains(&slug) {
		return None;
	}
	let chapter = match (parts.next(), parts.next()) {
		(Some("chapter"), Some(chapter)) if !chapter.is_empty() => Some(chapter.into()),
		_ => None,
	};
	Some((slug.into(), chapter))
}

pub fn card(link: &Element) -> Option<Manga> {
	let image = link.select_first("img")?;
	let (key, _) = series_path(&link.attr("href")?)?;
	let title = link
		.select_first("h3")
		.and_then(|heading| heading.text())
		.or_else(|| image.attr("alt"))
		.filter(|title| !title.is_empty())?;
	Some(Manga {
		key,
		title,
		cover: image.attr("abs:src"),
		content_rating: rating(image.has_attr("data-nsfw"), &[]),
		..Default::default()
	})
}

pub fn cards(links: Option<ElementList>) -> Vec<Manga> {
	let mut entries: Vec<Manga> = Vec::new();
	for link in links.into_iter().flatten() {
		if let Some(manga) = card(&link)
			&& !entries.iter().any(|entry| entry.key == manga.key)
		{
			entries.push(manga);
		}
	}
	entries
}

// `nsfw` is the site's blur flag on erotica and explicit covers.
pub fn rating(nsfw: bool, tags: &[String]) -> ContentRating {
	let has = |names: &[&str]| tags.iter().any(|tag| names.contains(&tag.as_str()));
	if nsfw || has(&["Adult", "Erotica", "Hentai", "Smut"]) {
		ContentRating::NSFW
	} else if has(&["Ecchi", "Mature"]) {
		ContentRating::Suggestive
	} else if tags.is_empty() {
		ContentRating::Unknown
	} else {
		ContentRating::Safe
	}
}

// Seconds since the site's "32m ago" style.
pub fn age(text: &str) -> Option<i64> {
	if text == "just now" {
		return Some(0);
	}
	let amount = text.strip_suffix(" ago")?;
	let split = amount.find(|c: char| !c.is_ascii_digit())?;
	let (number, unit) = amount.split_at(split);
	let unit = match unit {
		"m" => 60,
		"h" => 3600,
		"d" => 86400,
		"w" => 604_800,
		"mo" => 2_592_000,
		"y" => 31_536_000,
		_ => return None,
	};
	Some(number.parse::<i64>().ok()? * unit)
}

// The site's author slugs drop punctuation and join words with hyphens.
pub fn slugify(name: &str) -> String {
	let mut slug = String::new();
	for c in name.trim().to_lowercase().chars() {
		if c.is_ascii_alphanumeric() {
			slug.push(c);
		} else if (c.is_whitespace() || c == '-' || c == '_') && !slug.ends_with('-') {
			slug.push('-');
		}
	}
	slug.trim_matches('-').into()
}

pub fn series_page(key: &str) -> Result<Document> {
	Ok(Request::get(format!("{BASE_URL}/comic/{key}"))?.html()?)
}

pub fn comic_id(page: &Document) -> Result<String> {
	page.select_first("[data-comic-id]")
		.and_then(|marker| marker.attr("data-comic-id"))
		.ok_or_else(|| error!("Series not found"))
}

// Pages stream their content outside `main`, so selectors take the whole document.
pub fn details(page: &Document, manga: &mut Manga) -> Result<()> {
	manga.title = page
		.select_first("h1")
		.and_then(|heading| heading.text())
		.ok_or_else(|| error!("Series not found"))?;
	let nsfw = page
		.select_first("img[data-cover]")
		.is_some_and(|cover| cover.has_attr("data-nsfw"));
	let mut tags: Vec<String> = page
		.select("a[href^='/genre/']")
		.into_iter()
		.flatten()
		.filter_map(|link| link.text())
		.collect();
	tags.sort_unstable();
	tags.dedup();
	manga.cover = page
		.select_first("meta[property='og:image']")
		.and_then(|meta| meta.attr("content"));
	manga.description = page
		.select_first("p.whitespace-pre-line")
		.and_then(|text| text.text())
		.map(|text| text.escape_markdown());
	manga.url = page
		.select_first("link[rel=canonical]")
		.and_then(|link| link.attr("href"));
	manga.content_rating = rating(nsfw, &tags);
	manga.tags = Some(tags);
	for row in page.select("dl > div").into_iter().flatten() {
		let (Some(label), Some(value)) = (
			row.select_first("dt").and_then(|label| label.text()),
			row.select_first("dd"),
		) else {
			continue;
		};
		match label.as_str() {
			"Status" => {
				manga.status = match value.text().map(|text| text.to_lowercase()).as_deref() {
					Some("ongoing") => MangaStatus::Ongoing,
					Some("completed") => MangaStatus::Completed,
					Some("hiatus") => MangaStatus::Hiatus,
					Some("cancelled") => MangaStatus::Cancelled,
					_ => MangaStatus::Unknown,
				}
			}
			"Type" => {
				manga.viewer = match value.text().map(|text| text.to_lowercase()).as_deref() {
					Some("manhwa" | "manhua" | "webtoon") => Viewer::Webtoon,
					Some("manga") => Viewer::RightToLeft,
					Some("comic" | "western") => Viewer::LeftToRight,
					_ => Viewer::Unknown,
				}
			}
			"Author" | "Authors" => {
				let mut authors: Vec<String> = Vec::new();
				for name in value
					.select("a")
					.into_iter()
					.flatten()
					.filter_map(|a| a.text())
				{
					if !authors.contains(&name) {
						authors.push(name);
					}
				}
				manga.authors = Some(authors).filter(|authors| !authors.is_empty());
			}
			_ => {}
		}
	}
	Ok(())
}
