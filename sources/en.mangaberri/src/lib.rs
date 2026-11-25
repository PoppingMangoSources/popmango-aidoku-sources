#![no_std]
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, ImageRequestProvider, Link, Listing, ListingKind,
	ListingProvider, Manga, MangaPageResult, MangaStatus, Page, PageContent, PageContext, Result,
	Source, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
	prelude::*,
};

const BASE_URL: &str = "https://mangaberri.com";
const SORTS: [&str; 3] = ["latest", "popular", "title"];
const MATURE_GENRES: &[&str] = &["seinen", "ecchi", "harem", "mature", "adult", "smut"];

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

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

fn slug_from_href(href: &str) -> Option<String> {
	let after = href.split("/title/").nth(1)?;
	let slug = after.split(['/', '?', '#']).next()?;
	(!slug.is_empty()).then(|| slug.to_string())
}

fn chapter_from_href(href: &str) -> Option<(String, String)> {
	let after = href.split("/read/").nth(1)?;
	let mut parts = after.split(['/', '?', '#']);
	let slug = parts.next()?;
	let id = parts.next()?;
	(!slug.is_empty() && !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
		.then(|| (slug.to_string(), id.to_string()))
}

fn parse_status(value: &str) -> MangaStatus {
	match value.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("complet") || s.contains("finish") => MangaStatus::Completed,
		s if s.contains("hiatus") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("discontinu") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

fn content_rating_for(genres: &[String]) -> ContentRating {
	let mut suggestive = false;
	for genre in genres {
		let g = genre.trim().to_ascii_lowercase();
		if matches!(g.as_str(), "adult" | "smut" | "hentai") {
			return ContentRating::NSFW;
		}
		if MATURE_GENRES.contains(&g.as_str()) {
			suggestive = true;
		}
	}
	if suggestive {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

fn viewer_for(genres: &[String]) -> Viewer {
	if genres.iter().any(|g| {
		let lower = g.to_ascii_lowercase();
		lower.contains("manhwa") || lower.contains("manhua") || lower.contains("webtoon")
	}) {
		Viewer::Webtoon
	} else {
		Viewer::RightToLeft
	}
}

fn parse_chapter_label(name: &str) -> (Option<f32>, Option<String>) {
	let trimmed = name.trim();
	let lower = trimmed.to_ascii_lowercase();
	if let Some(rest) = lower
		.strip_prefix("chapter")
		.map(|_| trimmed["chapter".len()..].trim_start())
	{
		let mut end = 0;
		let mut seen_dot = false;
		for (i, ch) in rest.char_indices() {
			if ch.is_ascii_digit() {
				end = i + ch.len_utf8();
			} else if ch == '.' && !seen_dot && end > 0 {
				seen_dot = true;
				end = i + 1;
			} else {
				break;
			}
		}
		let num_str = rest[..end].trim_end_matches('.');
		if let Ok(num) = num_str.parse::<f32>() {
			let tail = rest
				.strip_prefix(num_str)
				.unwrap_or("")
				.trim_start_matches(|c: char| {
					c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—')
				})
				.trim();
			return (Some(num), (!tail.is_empty()).then(|| tail.to_string()));
		}
	}
	let mut digits = String::new();
	for ch in trimmed.chars() {
		if ch.is_ascii_digit() || (ch == '.' && !digits.is_empty()) {
			digits.push(ch);
		} else if !digits.is_empty() {
			break;
		}
	}
	let num = digits.trim_matches('.').parse::<f32>().ok();
	(num, (!trimmed.is_empty()).then(|| trimmed.to_string()))
}

fn parse_card(card: &Element) -> Option<Manga> {
	let href = card.select_first("a[href*='/title/']")?.attr("href")?;
	let key = slug_from_href(&href)?;
	let title = card
		.select_first("h3")
		.and_then(|el| el.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())?;
	let cover = card
		.select_first("img")
		.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
		.filter(|s| !s.is_empty())
		.map(|s| abs_url(&s));
	let status = card
		.select_first(".cover-label")
		.and_then(|el| el.text())
		.map(|t| parse_status(&t))
		.unwrap_or(MangaStatus::Unknown);
	let genres: Vec<String> = card
		.select(".card-meta span")
		.map(|els| {
			els.filter_map(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty() && !t.to_ascii_lowercase().contains("chapter"))
				.collect()
		})
		.unwrap_or_default();

	Some(Manga {
		url: Some(format!("{BASE_URL}/title/{key}")),
		key,
		title,
		cover,
		status,
		content_rating: content_rating_for(&genres),
		..Default::default()
	})
}

fn parse_cards_in(document: &Document, selector: &str) -> Vec<Manga> {
	let mut seen: Vec<String> = Vec::new();
	document
		.select(selector)
		.map(|cards| {
			cards
				.filter_map(|card| {
					let manga = parse_card(&card)?;
					if seen.contains(&manga.key) {
						return None;
					}
					seen.push(manga.key.clone());
					Some(manga)
				})
				.collect()
		})
		.unwrap_or_default()
}

fn has_next_page(document: &Document) -> bool {
	document.select("nav.pagination a").is_some_and(|links| {
		links.into_iter().any(|link| {
			link.text()
				.is_some_and(|text| text.to_ascii_lowercase().contains("next"))
		})
	})
}

fn fetch(url: &str) -> Result<Document> {
	Ok(Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()?)
}

fn make_listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}

struct MangaBerri;

impl Source for MangaBerri {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		if let Some(q) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
			qs.push("q", Some(q));
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						qs.set("sort", Some(sort));
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => {
					qs.set(&id, Some(&value));
				}
				_ => {}
			}
		}
		if page > 1 {
			qs.set("page", Some(&page.to_string()));
		}
		let url = if qs.is_empty() {
			format!("{BASE_URL}/browse")
		} else {
			format!("{BASE_URL}/browse?{qs}")
		};
		let document = fetch(&url)?;
		Ok(MangaPageResult {
			entries: parse_cards_in(&document, "[data-browse-results] article.manga-card"),
			has_next_page: has_next_page(&document),
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/title/{}", manga.key);
		let document = fetch(&url)?;

		if needs_details {
			if let Some(title) = document
				.select_first("section.detail h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = document
				.select_first("section.detail img.detail-cover")
				.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
				.filter(|s| !s.is_empty())
				.map(|s| abs_url(&s))
				.or_else(|| manga.cover.take());
			manga.description = document
				.select_first("section.detail p.synopsis")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			manga.authors = document
				.select_first("section.detail h1 + p.muted")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.map(|a| vec![a]);

			let mut genres: Vec<String> = Vec::new();
			if let Some(els) = document.select("section.detail .tags a") {
				for el in els {
					if let Some(tag) = el.text().map(|t| clean(&t)).filter(|t| !t.is_empty())
						&& !genres.iter().any(|g| g.eq_ignore_ascii_case(&tag))
					{
						genres.push(tag);
					}
				}
			}
			manga.content_rating = content_rating_for(&genres);
			manga.viewer = viewer_for(&genres);
			manga.tags = (!genres.is_empty()).then_some(genres);
			manga.status = document
				.select_first("section.detail .eyebrow")
				.and_then(|el| el.text())
				.map(|t| parse_status(t.split('·').next().unwrap_or(&t)))
				.unwrap_or(MangaStatus::Unknown);
			manga.url = Some(url);
		}

		if needs_chapters {
			let mut seen: Vec<String> = Vec::new();
			let chapters = document
				.select(".chapter-list a[data-chapter-group]")
				.map(|rows| {
					rows.filter_map(|row| {
						let href = row.attr("href")?;
						let (slug, id) = chapter_from_href(&href)?;
						if seen.contains(&id) {
							return None;
						}
						seen.push(id.clone());
						let raw_name = row
							.select_first("strong")
							.and_then(|el| el.text())
							.map(|t| clean(&t))
							.unwrap_or_default();
						let (chapter_number, title) = parse_chapter_label(&raw_name);
						let date_uploaded = row
							.select_first("span")
							.and_then(|el| el.text())
							.and_then(|t| {
								parse_date_with_options(
									clean(&t),
									"MMM d, yyyy",
									"en_US_POSIX",
									"UTC",
								)
							});
						let scanlators = (row.attr("data-chapter-group").as_deref() == Some("raw"))
							.then(|| vec!["RAW".into()]);
						Some(Chapter {
							url: Some(format!("{BASE_URL}/read/{slug}/{id}")),
							key: id,
							title,
							chapter_number,
							date_uploaded,
							scanlators,
							..Default::default()
						})
					})
					.collect()
				})
				.unwrap_or_default();
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/read/{}/{}", manga.key, chapter.key);
		let document = fetch(&url)?;
		let pages = document
			.select("img[data-page-image]")
			.map(|imgs| {
				imgs.filter_map(|img| {
					let src = img.attr("data-src").or_else(|| img.attr("src"))?;
					(!src.is_empty()).then(|| Page {
						content: PageContent::url(abs_url(&src)),
						..Default::default()
					})
				})
				.collect()
			})
			.unwrap_or_default();
		Ok(pages)
	}
}

impl ListingProvider for MangaBerri {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let sort = match listing.id.as_str() {
			"popular" | "latest" | "title" => listing.id.as_str(),
			_ => "latest",
		};
		let url = if page <= 1 {
			format!("{BASE_URL}/browse?sort={sort}")
		} else {
			format!("{BASE_URL}/browse?sort={sort}&page={page}")
		};
		let document = fetch(&url)?;
		Ok(MangaPageResult {
			entries: parse_cards_in(&document, "[data-browse-results] article.manga-card"),
			has_next_page: has_next_page(&document),
		})
	}
}

impl Home for MangaBerri {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			Request::get(format!("{BASE_URL}/discover"))?
				.header("Referer", &format!("{BASE_URL}/")),
			Request::get(format!("{BASE_URL}/browse?sort=popular"))?
				.header("Referer", &format!("{BASE_URL}/")),
		];
		let [discover_resp, popular_resp]: [_; 2] = Request::send_all(requests)
			.try_into()
			.map_err(|_| error!("Unexpected response count"))?;

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Ok(resp) = popular_resp
			&& let Ok(doc) = resp.get_html()
		{
			let entries: Vec<Manga> =
				parse_cards_in(&doc, "[data-browse-results] article.manga-card")
					.into_iter()
					.take(10)
					.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Read".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries,
						auto_scroll_interval: Some(6.0),
					},
				});
			}
		}

		if let Ok(resp) = discover_resp
			&& let Ok(doc) = resp.get_html()
		{
			let popular: Vec<Link> =
				parse_cards_in(&doc, "section.popular-manga article.manga-card")
					.into_iter()
					.map(Into::into)
					.collect();
			if !popular.is_empty() {
				components.push(HomeComponent {
					title: Some("Popular".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: popular,
						listing: Some(make_listing("popular", "Popular")),
					},
				});
			}

			let latest: Vec<Link> =
				parse_cards_in(&doc, "section:not(.popular-manga) article.manga-card")
					.into_iter()
					.map(Into::into)
					.collect();
			if !latest.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Updates".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: latest,
						listing: Some(make_listing("latest", "Latest Updates")),
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}

impl ImageRequestProvider for MangaBerri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaBerri {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if let Some((manga_key, key)) = chapter_from_href(&url) {
			return Ok(Some(DeepLinkResult::Chapter { manga_key, key }));
		}
		if let Some(key) = slug_from_href(&url) {
			return Ok(Some(DeepLinkResult::Manga { key }));
		}
		Ok(None)
	}
}

register_source!(
	MangaBerri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
