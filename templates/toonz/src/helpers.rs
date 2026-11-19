use crate::{Params, settings};
use aidoku::{
	ContentRating, Manga, MangaPageResult, MangaStatus, Result, Viewer,
	alloc::{String, Vec, format, string::ToString, vec},
	helpers::{
		string::PlainText,
		uri::{QueryParameters, encode_uri_component},
	},
	imports::{
		html::{Document, Element, ElementList},
		net::{Request, Response},
	},
	prelude::*,
};

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

// Each level shows its own titles and every level before it.
pub const RATINGS: [&str; 4] = ["safe", "suggestive", "erotica", "pornographic"];

// The site reads no types as every type; an unknown one matches nothing.
pub fn narrowed(types: Vec<String>, count: usize) -> Vec<String> {
	if types.is_empty() {
		vec!["none".into()]
	} else if types.len() < count {
		types
	} else {
		Vec::new()
	}
}

// Reports a refused request, such as an unsolved Cloudflare check, rather than parsing it.
pub fn check(response: Response) -> Result<Response> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("The site answered HTTP {status}");
	}
	Ok(response)
}

// The site's content setting, a cookie every page reads.
#[derive(Default)]
pub struct Content {
	pub rating: String,
	pub types: Vec<String>,
	pub blocked: Vec<String>,
}

impl Content {
	pub fn new(params: &Params) -> Self {
		Self {
			rating: settings::content_rating(params.rating),
			types: settings::types(params.type_count),
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

	pub fn html(&self, url: &str) -> Result<Document> {
		Ok(check(self.request(url)?.send()?)?.get_html()?)
	}
}

#[derive(Default)]
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
	pub fn new(params: &Params, query: Option<String>) -> Self {
		Self {
			query,
			sort: SORTS[0],
			content: Content::new(params),
			..Default::default()
		}
	}

	pub fn page(&self, params: &Params, page: i32) -> Result<MangaPageResult> {
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
			.html(&format!("{}/{}?{query}", params.base_url, params.catalog))?;
		Ok(MangaPageResult {
			entries: cards(params, document.select("img[data-cover]")),
			has_next_page: document
				.select_first("a[aria-label='Go to next page'][href]")
				.is_some(),
		})
	}
}

// Series keys are slugs, which the series path resolves for series and chapters alike.
pub fn series_path(params: &Params, path: &str) -> Option<(String, Option<String>)> {
	let mut parts = path.trim_matches('/').split(['/', '?', '#']);
	let kind = parts.next()?;
	let slug = parts.next().filter(|slug| !slug.is_empty())?;
	if !params.series_types.contains(&kind) || TYPE_PAGES.contains(&slug) {
		return None;
	}
	let chapter = match (parts.next(), parts.next()) {
		(Some("chapter"), Some(chapter)) if !chapter.is_empty() => Some(chapter.into()),
		_ => None,
	};
	Some((slug.into(), chapter))
}

// A cover's card is the nearest ancestor that is or holds a series link.
pub fn card(params: &Params, image: &Element) -> Option<(Element, Manga)> {
	let mut node = image.parent()?;
	for _ in 0..5 {
		let key = (node.has_attr("href"))
			.then(|| node.attr("href"))
			.flatten()
			.into_iter()
			.chain(
				node.select("a[href]")
					.into_iter()
					.flatten()
					.filter_map(|link| link.attr("href")),
			)
			.find_map(|href| series_path(params, &href));
		if let Some((key, _)) = key {
			let title = node
				.select_first("h3")
				.and_then(|heading| heading.text())
				.or_else(|| image.attr("alt"))
				.filter(|title| !title.is_empty())?;
			let manga = Manga {
				key,
				title,
				cover: image.attr("abs:src"),
				content_rating: rating(image.has_attr("data-nsfw"), &[]),
				..Default::default()
			};
			return Some((node, manga));
		}
		node = node.parent()?;
	}
	None
}

pub fn cards(params: &Params, images: Option<ElementList>) -> Vec<Manga> {
	let mut entries: Vec<Manga> = Vec::new();
	for image in images.into_iter().flatten() {
		if let Some((_, manga)) = card(params, &image)
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

pub fn status(text: &str) -> Option<MangaStatus> {
	Some(match text.to_lowercase().as_str() {
		"ongoing" => MangaStatus::Ongoing,
		"completed" => MangaStatus::Completed,
		"hiatus" => MangaStatus::Hiatus,
		"cancelled" => MangaStatus::Cancelled,
		_ => return None,
	})
}

fn viewer(text: &str) -> Option<Viewer> {
	Some(match text.to_lowercase().as_str() {
		"manhwa" | "manhua" | "webtoon" => Viewer::Webtoon,
		"manga" => Viewer::RightToLeft,
		"comic" | "western" => Viewer::LeftToRight,
		_ => return None,
	})
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

pub fn series_page(params: &Params, key: &str) -> Result<Document> {
	let url = format!("{}/{}/{key}", params.base_url, params.series_path);
	Ok(check(Request::get(url)?.send()?)?.get_html()?)
}

pub fn comic_id(page: &Document) -> Result<String> {
	page.select_first("[data-comic-id]")
		.and_then(|marker| marker.attr("data-comic-id"))
		.ok_or_else(|| error!("Series not found"))
}

fn texts(list: Option<ElementList>) -> impl Iterator<Item = String> {
	list.into_iter()
		.flatten()
		.filter_map(|element| element.text())
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
	let mut tags: Vec<String> = texts(page.select("a[href^='/genre/']")).collect();
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

	// One layout lists these in a table, the other as badges and author links.
	let mut status_text = None;
	let mut type_text = None;
	let mut author_links = page.select("a[href^='/author/']");
	for row in page.select("dl > div").into_iter().flatten() {
		let (Some(label), Some(value)) = (
			row.select_first("dt").and_then(|label| label.text()),
			row.select_first("dd"),
		) else {
			continue;
		};
		match label.as_str() {
			"Status" => status_text = value.text(),
			"Type" => type_text = value.text(),
			"Author" | "Authors" => author_links = value.select("a"),
			_ => {}
		}
	}
	let badges: Vec<String> = texts(page.select("[data-slot=badge]")).collect();
	manga.status = status_text
		.as_deref()
		.and_then(status)
		.or_else(|| badges.iter().find_map(|badge| status(badge)))
		.unwrap_or_default();
	manga.viewer = type_text
		.as_deref()
		.and_then(viewer)
		.or_else(|| badges.iter().find_map(|badge| viewer(badge)))
		.unwrap_or_default();
	let mut authors: Vec<String> = Vec::new();
	for name in texts(author_links) {
		if !authors.contains(&name) {
			authors.push(name);
		}
	}
	manga.authors = Some(authors).filter(|authors| !authors.is_empty());
	Ok(())
}
