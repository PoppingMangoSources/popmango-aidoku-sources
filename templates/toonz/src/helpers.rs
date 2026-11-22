use crate::{
	Params,
	models::{ChapterItem, ChapterList, FeaturedSeries, Genre, ReaderImage},
	settings,
};
use aidoku::{
	ContentRating, Manga, MangaPageResult, MangaStatus, Result, Viewer,
	alloc::{String, Vec, borrow::Cow, collections::BTreeSet, format, string::ToString, vec},
	helpers::uri::{QueryParameters, encode_uri_component},
	imports::{
		html::{Document, Element, ElementList},
		net::{Request, Response},
	},
	prelude::*,
};
use serde::Serialize;

const TYPE_PAGES: [&str; 5] = [
	"browse",
	"top-rated",
	"most-popular",
	"trending",
	"most-favorited",
];

pub const SORTS: [&str; 4] = ["latest", "popular", "rating", "a-z"];

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

pub fn check_response(response: Response) -> Result<Response> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		bail!("The site answered HTTP {status}");
	}
	Ok(response)
}

#[derive(Default, Serialize)]
pub struct Content {
	pub rating: String,
	pub types: Vec<String>,
	#[serde(rename = "blockedGenres")]
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
		let prefs = serde_json::to_string(self)?;
		Ok(Request::get(url)?.header(
			"Cookie",
			&format!("reader_prefs={}", encode_uri_component(prefs)),
		))
	}
}

#[derive(Default)]
pub struct SearchParams {
	pub query: Option<String>,
	pub sort: &'static str,
	pub status: Option<String>,
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
		if self.content.types.as_slice() == ["none"] {
			return Ok(MangaPageResult::default());
		}
		let mut query = QueryParameters::new();
		if self.query.is_some() {
			query.push("search", self.query.as_deref());
		}
		query.push_encoded("sort", Some(self.sort));
		if self.status.is_some() {
			query.push_encoded("status", self.status.as_deref());
		}
		if let Some(author) = &self.author {
			query.push("author", Some(&slugify(author)));
		}
		if !self.genres.is_empty() {
			query.push("genres", Some(&self.genres.join(",")));
		}
		if !self.excluded_genres.is_empty() {
			query.push("excludeGenres", Some(&self.excluded_genres.join(",")));
		}
		query.push_encoded("page", Some(&page.to_string()));
		let url = format!("{}/{}?{query}", params.base_url, params.catalog);
		let document = check_response(self.content.request(&url)?.send()?)?.get_html()?;
		Ok(MangaPageResult {
			entries: cards(params, document.select("img[data-cover]")),
			has_next_page: document
				.select_first("a[aria-label='Go to next page'][href]")
				.is_some(),
		})
	}
}

pub fn parse_series_path<'a>(params: &Params, path: &'a str) -> Option<(&'a str, Option<&'a str>)> {
	let mut parts = path
		.strip_prefix('/')?
		.split(['?', '#'])
		.next()?
		.trim_end_matches('/')
		.split('/');
	let kind = parts.next()?;
	let slug = parts.next().filter(|slug| !slug.is_empty())?;
	if !params.series_types.contains(&kind) || TYPE_PAGES.contains(&slug) {
		return None;
	}
	let chapter = match (parts.next(), parts.next(), parts.next()) {
		(None, None, None) => None,
		(Some("chapter"), Some(chapter), None) if !chapter.is_empty() => Some(chapter),
		_ => return None,
	};
	Some((slug, chapter))
}

// LewdManhwa's series link is an empty overlay beside the cover, not around it.
pub fn card(params: &Params, image: &Element) -> Option<(Element, Manga)> {
	let mut node = image.parent()?;
	for _ in 0..5 {
		let key = node
			.attr("href")
			.into_iter()
			.chain(
				node.select("a[href]")
					.into_iter()
					.flatten()
					.filter_map(|link| link.attr("href")),
			)
			.find_map(|href| parse_series_path(params, &href).map(|(key, _)| String::from(key)));
		if let Some(key) = key {
			let title = node
				.select_first("h3")
				.and_then(|heading| heading.text())
				.filter(|title| !title.is_empty())
				.or_else(|| {
					let title = image.attr("alt")?;
					let title = title.trim();
					(!title.is_empty()).then(|| title.into())
				})?;
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
	let mut seen = BTreeSet::new();
	images
		.into_iter()
		.flatten()
		.filter_map(|image| {
			let (_, manga) = card(params, &image)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect()
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
	number.parse::<i64>().ok()?.checked_mul(unit)
}

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

pub fn chapter_url(params: &Params, manga_key: &str, key: &str) -> String {
	format!(
		"{}/{}/{manga_key}/chapter/{key}",
		params.base_url, params.series_path
	)
}

pub fn series_page(params: &Params, key: &str) -> Result<Document> {
	let url = format!("{}/{}/{key}", params.base_url, params.series_path);
	Ok(check_response(Request::get(url)?.send()?)?.get_html()?)
}

pub(crate) fn comic_id(page: &Document) -> Result<String> {
	page.select_first("[data-comic-id]")
		.and_then(|marker| marker.attr("data-comic-id"))
		.filter(|id| !id.trim().is_empty())
		.ok_or_else(|| error!("Series not found"))
}

pub fn genres(params: &Params) -> Result<Vec<Genre>> {
	let url = format!("{}/api/genres", params.base_url);
	check_response(Request::get(url)?.send()?)?.get_json_owned()
}

type GenreOptions = (Vec<Cow<'static, str>>, Vec<Cow<'static, str>>);

pub fn genre_options(params: &Params) -> Result<GenreOptions> {
	Ok(genres(params)?
		.into_iter()
		.map(|genre| (genre.name.into(), genre.slug.into()))
		.unzip())
}

pub fn chapter_items(params: &Params, page: &Document) -> Result<Vec<ChapterItem>> {
	let url = format!(
		"{}/api/comics/{}/chapters",
		params.base_url,
		comic_id(page)?
	);
	Ok(check_response(Request::get(url)?.send()?)?
		.get_json_owned::<ChapterList>()?
		.chapters)
}

pub fn reader_images(
	params: &Params,
	manga_key: &str,
	chapter_key: &str,
) -> Result<Option<Vec<ReaderImage>>> {
	let response = Request::get(chapter_url(params, manga_key, chapter_key))?
		.header("RSC", "1")
		.send()?;
	if response.status_code() == 404 {
		return Ok(None);
	}
	let body = check_response(response)?.get_string()?;
	// The first "images" array that reads as pages; another can come first.
	let marker = r#""images":"#;
	body.match_indices(marker)
		.find_map(|(start, _)| {
			serde_json::Deserializer::from_str(&body[start + marker.len()..])
				.into_iter::<Vec<ReaderImage>>()
				.next()?
				.ok()
				.filter(|images| !images.is_empty())
		})
		.map(Some)
		.ok_or_else(|| error!("This chapter has missing pages"))
}

pub fn featured_series(page: &Document, property: &str) -> Vec<FeaturedSeries> {
	let mut flight = String::new();
	// A slide record can span several script chunks.
	for data in page
		.select("script:not([src])")
		.into_iter()
		.flatten()
		.filter_map(|script| script.data().or_else(|| script.html()))
	{
		let Some(chunk) = data.strip_prefix("self.__next_f.push(") else {
			continue;
		};
		if let Some(Ok((1, text))) = serde_json::Deserializer::from_str(chunk)
			.into_iter::<(u8, String)>()
			.next()
		{
			flight.push_str(&text);
		}
	}
	flight
		.match_indices(property)
		.find_map(|(start, _)| {
			serde_json::Deserializer::from_str(&flight[start + property.len()..])
				.into_iter::<Vec<FeaturedSeries>>()
				.next()?
				.ok()
				.filter(|entries| !entries.is_empty())
		})
		.unwrap_or_default()
}

pub fn cover_url(params: &Params, featured: Option<&FeaturedSeries>) -> Option<String> {
	let path = featured?.cover.as_deref()?.trim();
	(!path.is_empty()).then(|| format!("{}/uploads/{path}", params.base_url))
}

fn texts(list: Option<ElementList>) -> impl Iterator<Item = String> {
	list.into_iter()
		.flatten()
		.filter_map(|element| element.text())
}

pub fn series_summary(page: &Document) -> Result<(String, Option<String>)> {
	// Next.js can stream the series content outside <main>.
	let title = page
		.select_first("h1")
		.and_then(|heading| heading.text())
		.filter(|title| !title.is_empty())
		.ok_or_else(|| error!("Series not found"))?;
	let cover = page
		.select_first("meta[property='og:image']")
		.and_then(|meta| meta.attr("content"));
	Ok((title, cover))
}

pub fn details(page: &Document, manga: &mut Manga) -> Result<()> {
	(manga.title, manga.cover) = series_summary(page)?;
	let nsfw = page
		.select_first("img[data-cover]")
		.is_some_and(|cover| cover.has_attr("data-nsfw"));
	let tags = tags(page.select("a[href^='/genre/']"));
	manga.description = page
		.select_first("p.whitespace-pre-line")
		.and_then(|text| text.untrimmed_text())
		.and_then(|text| {
			let text = text.trim();
			(!text.is_empty()).then(|| text.into())
		});
	manga.url = page
		.select_first("link[rel=canonical]")
		.and_then(|link| link.attr("href"));
	manga.content_rating = rating(nsfw, &tags);
	manga.tags = Some(tags).filter(|tags| !tags.is_empty());

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
	for name in texts(author_links).filter(|name| !name.is_empty()) {
		if !authors.contains(&name) {
			authors.push(name);
		}
	}
	manga.authors = Some(authors).filter(|authors| !authors.is_empty());
	Ok(())
}

pub fn tags(elements: Option<ElementList>) -> Vec<String> {
	let mut tags: Vec<_> = texts(elements).filter(|tag| !tag.is_empty()).collect();
	tags.sort_unstable();
	tags.dedup();
	tags
}
