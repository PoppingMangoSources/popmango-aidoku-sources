#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, Link, Listing, ListingProvider, Manga, MangaPageResult,
	MangaWithChapter, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::uri::encode_uri_component,
	imports::{
		html::{Document, Element},
		net::Request,
		std::current_date,
	},
	prelude::*,
};

const BASE_URL: &str = "https://mangacherri.com";

const MATURE_GENRES: &[&str] = &["seinen", "ecchi", "harem", "mature", "adult", "smut"];

fn abs_url(url: &str) -> String {
	if url.is_empty() {
		String::new()
	} else if url.starts_with("http") {
		url.to_string()
	} else if url.starts_with('/') {
		format!("{BASE_URL}{url}")
	} else {
		format!("{BASE_URL}/{url}")
	}
}

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The manga id is the first path segment of a `/slug` link.
fn slug_from_href(href: &str) -> String {
	href.trim_start_matches('/')
		.split(['/', '?', '#'])
		.next()
		.unwrap_or("")
		.to_string()
}

/// Chapter links look like `/slug/12345`; the trailing number is the id.
fn chapter_id_from_href(href: &str) -> Option<String> {
	let mut segments = href.trim_start_matches('/').split(['/', '?', '#']);
	segments.next()?; // slug
	let id = segments.next()?;
	(!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| id.to_string())
}

fn is_slug_only(href: &str) -> bool {
	let trimmed = href.trim_start_matches('/');
	let mut segments = trimmed.split(['?', '#']).next().unwrap_or("").split('/');
	segments.next().is_some_and(|first| !first.is_empty())
		&& segments.all(|segment| segment.is_empty())
}

fn cover_from(element: &Element) -> Option<String> {
	let img = element.select_first("img")?;
	let src = img
		.attr("data-src")
		.or_else(|| img.attr("src"))
		.unwrap_or_default();
	(!src.is_empty()).then(|| abs_url(&src))
}

fn content_rating_for(genres: &[String]) -> ContentRating {
	let lowered: Vec<String> = genres.iter().map(|g| g.to_lowercase()).collect();
	if lowered.iter().any(|g| MATURE_GENRES.contains(&g.as_str())) {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

fn chapter_number(text: &str) -> Option<f32> {
	let mut number = String::new();
	for ch in text.chars() {
		if ch.is_ascii_digit() || (ch == '.' && !number.is_empty()) {
			number.push(ch);
		} else if !number.is_empty() {
			break;
		}
	}
	number.trim_matches('.').parse().ok()
}

/// Update labels are relative ("13 hours 49 mins ago", "7 days ago"); sum the
/// units back from now.
fn parse_relative_date(text: &str) -> Option<i64> {
	let lowered = text.trim().to_lowercase();
	if lowered.is_empty() {
		return None;
	}
	if lowered.contains("just now") || lowered.contains("less than") {
		return Some(current_date());
	}
	let units: [(&str, i64); 7] = [
		("sec", 1),
		("min", 60),
		("hour", 3600),
		("day", 86400),
		("week", 604800),
		("month", 2629800),
		("year", 31557600),
	];
	let mut offset: i64 = 0;
	let bytes = lowered.as_bytes();
	for (unit, factor) in units {
		if let Some(pos) = lowered.find(unit) {
			// Walk back over spaces then digits to read the amount for this unit.
			let mut end = pos;
			while end > 0 && bytes[end - 1] == b' ' {
				end -= 1;
			}
			let mut start = end;
			while start > 0 && bytes[start - 1].is_ascii_digit() {
				start -= 1;
			}
			if start < end
				&& let Ok(amount) = lowered[start..end].parse::<i64>()
			{
				offset += amount * factor;
			}
		}
	}
	(offset > 0).then(|| current_date() - offset)
}

/// Reads a card as it appears in a listing or carousel.
fn parse_card(item: &Element) -> Option<Manga> {
	let anchors: Vec<Element> = item.select("a[href]").map(|els| els.collect())?;
	let slug_link = anchors
		.iter()
		.find(|a| a.attr("href").is_some_and(|href| is_slug_only(&href)))
		.or_else(|| anchors.first())?;
	let slug = slug_from_href(&slug_link.attr("href")?);
	if slug.is_empty() {
		return None;
	}
	let title = anchors
		.iter()
		.filter(|a| a.attr("href").is_some_and(|href| is_slug_only(&href)))
		.find(|a| a.select_first("img").is_none())
		.and_then(|a| a.text())
		.map(|text| clean(&text))
		.filter(|text| !text.is_empty())
		.or_else(|| slug_link.attr("title").map(|t| clean(&t)))?;

	Some(Manga {
		key: slug.clone(),
		title,
		cover: cover_from(item),
		url: Some(format!("{BASE_URL}/{slug}")),
		content_rating: ContentRating::Suggestive,
		..Default::default()
	})
}

fn latest_from(item: &Element) -> Option<(String, String)> {
	let chapter = item.select("a[href]")?.find(|a| {
		a.attr("href")
			.is_some_and(|href| chapter_id_from_href(&href).is_some())
	})?;
	let href = chapter.attr("href")?;
	let id = chapter_id_from_href(&href)?;
	let label = chapter.text().map(|text| clean(&text)).unwrap_or_default();
	Some((id, label))
}

fn detail_field(document: &Document, label: &str) -> Option<String> {
	let rows = document.select(".section-status.row, .comic-attrs .column")?;
	for row in rows {
		let children: Vec<Element> = row.children().collect();
		let key = children
			.first()
			.and_then(|el| el.text())
			.map(|text| clean(&text))
			.unwrap_or_default();
		if key.eq_ignore_ascii_case(label) {
			return children
				.last()
				.and_then(|el| el.text())
				.map(|text| clean(&text));
		}
	}
	None
}

fn parse_details(document: &Document, manga: &mut Manga) {
	if let Some(title) = document
		.select_first("h1.story-name")
		.and_then(|el| el.text())
		.map(|text| clean(&text))
		.filter(|text| !text.is_empty())
	{
		manga.title = title;
	}
	manga.cover = document
		.select_first("img.comic-img")
		.and_then(|img| img.attr("src"))
		.map(|src| abs_url(&src))
		.or_else(|| manga.cover.take());
	manga.description = document
		.select_first(".story-desc")
		.and_then(|el| el.text())
		.map(|text| clean(&text))
		.filter(|text| !text.is_empty());
	manga.authors = document
		.select_first(".comic-attrs a[href*='/author/']")
		.and_then(|el| el.text())
		.map(|text| vec![clean(&text)]);

	let genres: Vec<String> = document
		.select(".comic-attrs a[href*='genre.php?genre=']")
		.map(|els| {
			els.filter_map(|el| el.text())
				.map(|text| clean(&text))
				.filter(|text| !text.is_empty())
				.collect()
		})
		.unwrap_or_default();
	manga.content_rating = content_rating_for(&genres);
	manga.tags = (!genres.is_empty()).then_some(genres);

	manga.status = match detail_field(document, "Status")
		.unwrap_or_default()
		.to_lowercase()
		.as_str()
	{
		s if s.contains("ongoing") => aidoku::MangaStatus::Ongoing,
		s if s.contains("complet") => aidoku::MangaStatus::Completed,
		s if s.contains("hiatus") => aidoku::MangaStatus::Hiatus,
		s if s.contains("cancel") => aidoku::MangaStatus::Cancelled,
		_ => aidoku::MangaStatus::Unknown,
	};
	manga.url = Some(format!("{BASE_URL}/{}", manga.key));
}

fn parse_chapters(document: &Document) -> Vec<Chapter> {
	let Some(anchors) = document.select(".chapters-container a[href]") else {
		return Vec::new();
	};
	let mut seen: Vec<String> = Vec::new();
	let entries: Vec<(String, String)> = anchors
		.filter_map(|a| {
			let href = a.attr("href")?;
			let id = chapter_id_from_href(&href)?;
			let label = a.text().map(|text| clean(&text)).unwrap_or_default();
			Some((id, label))
		})
		.collect();
	let total = entries.len();
	entries
		.into_iter()
		.enumerate()
		.filter_map(|(index, (id, label))| {
			if seen.contains(&id) {
				return None;
			}
			seen.push(id.clone());
			Some(Chapter {
				chapter_number: chapter_number(&label).or(Some((total - index) as f32)),
				title: (!label.is_empty()).then(|| label.clone()),
				url: Some(format!("{BASE_URL}/{id}")),
				key: id,
				..Default::default()
			})
		})
		.collect()
}

fn fetch(url: &str) -> Result<Document> {
	Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()
		.map_err(Into::into)
}

fn cards_from(document: &Document, selector: &str) -> Vec<Manga> {
	document
		.select(selector)
		.map(|items| items.filter_map(|item| parse_card(&item)).collect())
		.unwrap_or_default()
}

/// The `.section-container` that holds the `.section-title` whose text matches
/// `title` — the equivalent of the source's `.section-title … .closest()`.
fn find_section(document: &Document, title: &str) -> Option<Element> {
	let titles = document.select(".section-title")?;
	for heading in titles {
		let text = heading.text().map(|t| clean(&t)).unwrap_or_default();
		if !text.eq_ignore_ascii_case(title) {
			continue;
		}
		let mut current = heading.parent();
		while let Some(element) = current {
			if element.has_class("section-container") {
				return Some(element);
			}
			current = element.parent();
		}
	}
	None
}

/// A home-carousel card: a cover link, a title link and a rating badge.
fn carousel_card(item: &Element) -> Option<Manga> {
	let slug = slug_from_href(&item.select_first("a.manga-cover-link")?.attr("href")?);
	if slug.is_empty() {
		return None;
	}
	let title = item
		.select_first("a.manga-title-link")
		.and_then(|a| a.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())?;
	let cover = item
		.select_first(".manga-live-cover img, img")
		.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
		.map(|src| abs_url(&src))
		.filter(|url| !url.is_empty());
	Some(Manga {
		key: slug.clone(),
		title,
		cover,
		url: Some(format!("{BASE_URL}/{slug}")),
		content_rating: ContentRating::Suggestive,
		..Default::default()
	})
}

/// The carousel cards under the section titled `title`.
fn carousel_cards(document: &Document, title: &str) -> Vec<Manga> {
	find_section(document, title)
		.and_then(|section| section.select(".manga-item.manga-live-card, .manga-item"))
		.map(|items| items.filter_map(|item| carousel_card(&item)).collect())
		.unwrap_or_default()
}

/// The "Latest Chapter" rail: each card carries its newest chapter and a
/// relative timestamp.
fn latest_section(document: &Document) -> Vec<MangaWithChapter> {
	let Some(section) = find_section(document, "Latest Chapter") else {
		return Vec::new();
	};
	let Some(items) = section.select(".manga-horizontal-item") else {
		return Vec::new();
	};
	items
		.filter_map(|item| {
			let manga = carousel_card(&item)?;
			let (id, label) = latest_from(&item)?;
			let date = item
				.select_first(".episode-date")
				.and_then(|el| el.text())
				.and_then(|t| parse_relative_date(&t));
			Some(MangaWithChapter {
				manga,
				chapter: Chapter {
					chapter_number: chapter_number(&label),
					date_uploaded: date,
					title: (!label.is_empty()).then_some(label),
					url: Some(format!("{BASE_URL}/{id}")),
					key: id,
					..Default::default()
				},
			})
		})
		.collect()
}

fn genre_url(name: &str) -> String {
	format!(
		"{BASE_URL}/genre.php?genre={}",
		encode_uri_component(name).replace("%2F", "/")
	)
}

struct MangaCherri;

impl Source for MangaCherri {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		_page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.unwrap_or_default();
		let query = query.trim();
		let genre = filters.into_iter().find_map(|filter| match filter {
			FilterValue::Select { id, value } if id == "genre" && value != "all" => Some(value),
			_ => None,
		});

		let url = if !query.is_empty() {
			format!(
				"{BASE_URL}/search.php?keyword={}",
				encode_uri_component(query)
			)
		} else if let Some(genre) = genre {
			genre_url(&genre)
		} else {
			format!("{BASE_URL}/home.php")
		};

		let document = fetch(&url)?;
		let entries = cards_from(&document, ".manga-item, .manga-horizontal-item");
		Ok(MangaPageResult {
			entries,
			has_next_page: false,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let document = fetch(&format!("{BASE_URL}/{}", manga.key))?;
		if needs_details {
			parse_details(&document, &mut manga);
		}
		if needs_chapters {
			manga.chapters = Some(parse_chapters(&document));
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let document = fetch(&format!("{BASE_URL}/{}/{}", manga.key, chapter.key))?;
		let mut seen: Vec<String> = Vec::new();
		let pages = document
			.select(".reading-container img")
			.map(|images| {
				images
					.filter_map(|img| {
						let src = img.attr("data-src").or_else(|| img.attr("src"))?;
						let url = abs_url(&src);
						(!url.is_empty() && !seen.contains(&url)).then(|| {
							seen.push(url.clone());
							Page {
								content: PageContent::url(url),
								..Default::default()
							}
						})
					})
					.collect()
			})
			.unwrap_or_default();
		Ok(pages)
	}
}

impl Home for MangaCherri {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			format!("{BASE_URL}/home.php"),
			format!("{BASE_URL}/weekly-manga.php"),
		]
		.into_iter()
		.map(|url| {
			Request::get(&url)
				.map(|r| r.header("Referer", &format!("{BASE_URL}/")))
				.map_err(Into::into)
		})
		.collect::<Result<Vec<_>>>()?;
		let mut documents = Request::send_all(requests)
			.into_iter()
			.map(|response| response.ok().and_then(|response| response.get_html().ok()));

		let home = documents.next().flatten();
		let weekly = documents.next().flatten();

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Some(home) = home.as_ref() {
			let popular = carousel_cards(home, "Most Popular");
			if !popular.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Popular".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries: popular.into_iter().take(10).collect(),
						auto_scroll_interval: Some(6.0),
					},
				});
			}

			let popular_now = carousel_cards(home, "Popular Now");
			if !popular_now.is_empty() {
				components.push(HomeComponent {
					title: Some("Popular Now".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries: popular_now.into_iter().map(Into::into).collect(),
						listing: None,
					},
				});
			}

			let latest = latest_section(home);
			if !latest.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Chapter".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: None,
						entries: latest,
						listing: None,
					},
				});
			}

			let completed = carousel_cards(home, "Completed Romance Manga");
			if !completed.is_empty() {
				components.push(HomeComponent {
					title: Some("Completed Romance".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: completed.into_iter().map(Into::into).collect(),
						listing: None,
					},
				});
			}
		}

		if let Some(weekly) = weekly.as_ref() {
			let entries: Vec<Link> = cards_from(weekly, ".manga-item")
				.into_iter()
				.take(30)
				.map(Into::into)
				.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Top Weekly".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					},
				});
			}
		}

		// Safety net: if the headings ever change and no shelf matched, show every
		// card on the home page so the source is never blank.
		if components.is_empty()
			&& let Some(home) = home.as_ref()
		{
			let mut entries: Vec<Manga> = carousel_cards(home, "Most Popular");
			if entries.is_empty() {
				entries = home
					.select(".manga-item.manga-live-card, .manga-horizontal-item, .manga-item")
					.map(|items| {
						items
							.filter_map(|item| carousel_card(&item).or_else(|| parse_card(&item)))
							.collect()
					})
					.unwrap_or_default();
			}
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Browse".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: entries.into_iter().map(Into::into).collect(),
						listing: None,
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}

impl ListingProvider for MangaCherri {
	fn get_manga_list(&self, listing: Listing, _page: i32) -> Result<MangaPageResult> {
		let url = match listing.id.as_str() {
			"weekly" => format!("{BASE_URL}/weekly-manga.php"),
			_ => format!("{BASE_URL}/home.php"),
		};
		let document = fetch(&url)?;
		Ok(MangaPageResult {
			entries: cards_from(&document, ".manga-item, .manga-horizontal-item"),
			has_next_page: false,
		})
	}
}

impl aidoku::ImageRequestProvider for MangaCherri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaCherri {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = url.strip_prefix(BASE_URL).unwrap_or(&url);
		let slug = slug_from_href(path);
		if slug.is_empty() || slug.ends_with(".php") {
			return Ok(None);
		}
		Ok(Some(DeepLinkResult::Manga { key: slug }))
	}
}

register_source!(
	MangaCherri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
