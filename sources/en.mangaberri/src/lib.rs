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
	},
	prelude::*,
};

// SITE is swapped per source (mangaberri.com / mangacherri.com).
const BASE_URL: &str = "https://mangaberri.com";

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

/// The cards in the section whose `.section-title` matches `title`.
fn section_cards(document: &Document, title: &str, item_selector: &str) -> Vec<Manga> {
	let Some(sections) = document.select(".section-container") else {
		return Vec::new();
	};
	for section in sections {
		let heading = section
			.select_first(".section-title")
			.and_then(|el| el.text())
			.map(|text| clean(&text))
			.unwrap_or_default();
		if heading.eq_ignore_ascii_case(title) {
			return section
				.select(item_selector)
				.map(|items| items.filter_map(|item| parse_card(&item)).collect())
				.unwrap_or_default();
		}
	}
	Vec::new()
}

fn genre_url(name: &str) -> String {
	format!(
		"{BASE_URL}/genre.php?genre={}",
		encode_uri_component(name).replace("%2F", "/")
	)
}

struct MangaBerri;

impl Source for MangaBerri {
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

impl Home for MangaBerri {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			format!("{BASE_URL}/home.php"),
			format!("{BASE_URL}/weekly-manga.php"),
			genre_url("Shounen"),
			genre_url("Seinen"),
			genre_url("Manhwa/Manhua"),
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
		let shounen = documents.next().flatten();
		let seinen = documents.next().flatten();
		let manhwa = documents.next().flatten();

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Some(home) = home.as_ref() {
			let viewed = section_cards(home, "Most Viewed", ".manga-item");
			if !viewed.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Viewed".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries: viewed.into_iter().take(10).collect(),
						auto_scroll_interval: Some(6.0),
					},
				});
			}

			let latest: Vec<MangaWithChapter> = section_cards(home, "Latest Update", "")
				.into_iter()
				.collect::<Vec<_>>()
				.into_iter()
				.map(|manga| MangaWithChapter {
					manga,
					chapter: Chapter::default(),
				})
				.collect();
			let latest_items = home
				.select(".section-container")
				.and_then(|sections| {
					sections.into_iter().find(|section| {
						section
							.select_first(".section-title")
							.and_then(|el| el.text())
							.map(|text| clean(&text))
							.is_some_and(|t| t.eq_ignore_ascii_case("Latest Update"))
					})
				})
				.and_then(|section| section.select(".manga-horizontal-item"));
			let latest: Vec<MangaWithChapter> = latest_items
				.map(|items| {
					items
						.filter_map(|item| {
							let manga = parse_card(&item)?;
							let (id, label) = latest_from(&item).unwrap_or_default();
							Some(MangaWithChapter {
								manga,
								chapter: Chapter {
									chapter_number: chapter_number(&label),
									title: (!label.is_empty()).then_some(label),
									url: (!id.is_empty()).then(|| format!("{BASE_URL}/{id}")),
									key: id,
									..Default::default()
								},
							})
						})
						.collect()
				})
				.unwrap_or(latest);
			if !latest.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Update".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: None,
						entries: latest,
						listing: None,
					},
				});
			}

			let popular = section_cards(home, "Popular Today", ".manga-item");
			if !popular.is_empty() {
				components.push(HomeComponent {
					title: Some("Popular Today".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: popular.into_iter().map(Into::into).collect(),
						listing: None,
					},
				});
			}
		}

		for (title, document, ranked) in [
			("Top Weekly", weekly, true),
			("Top Shounen", shounen, true),
			("Top Seinen", seinen, true),
			("Manhwa / Manhua", manhwa, false),
		] {
			let Some(document) = document else { continue };
			let entries: Vec<Link> = cards_from(&document, ".manga-item, .manga-horizontal-item")
				.into_iter()
				.take(30)
				.map(Into::into)
				.collect();
			if entries.is_empty() {
				continue;
			}
			components.push(HomeComponent {
				title: Some(title.into()),
				subtitle: None,
				value: if ranked {
					HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					}
				} else {
					HomeComponentValue::Scroller {
						entries,
						listing: None,
					}
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

impl ListingProvider for MangaBerri {
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

impl aidoku::ImageRequestProvider for MangaBerri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaBerri {
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
	MangaBerri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
