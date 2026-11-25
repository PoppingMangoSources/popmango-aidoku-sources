#![no_std]
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, ImageRequestProvider, Link, Listing, ListingKind,
	ListingProvider, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Page, PageContent,
	PageContext, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
	prelude::*,
};

const BASE_URL: &str = "https://mangacherri.com";
const SORTS: [&str; 6] = [
	"updated",
	"added",
	"popular",
	"chapters",
	"title",
	"title_desc",
];
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

fn chapter_id_from_href(href: &str) -> Option<String> {
	let after = href.split("/read/").nth(1)?;
	let id = after.split(['/', '?', '#']).next()?;
	(!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| id.to_string())
}

fn parse_status(value: &str) -> MangaStatus {
	match value.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("complet") => MangaStatus::Completed,
		s if s.contains("hiatus") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("drop") => MangaStatus::Cancelled,
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

fn parse_iso_utc(raw: &str) -> Option<i64> {
	let trimmed = raw.trim();
	if trimmed.len() >= 19 {
		let formatted = trimmed[..19].replace('T', " ");
		parse_date_with_options(formatted, "yyyy-MM-dd HH:mm:ss", "en_US_POSIX", "UTC")
	} else if trimmed.len() >= 10 {
		parse_date_with_options(&trimmed[..10], "yyyy-MM-dd", "en_US_POSIX", "UTC")
	} else {
		None
	}
}

fn parse_chapter_label(name: &str) -> (Option<f32>, Option<String>) {
	let trimmed = name.trim();
	let lower = trimmed.to_ascii_lowercase();
	for prefix in ["chapter", "ch.", "ch"] {
		if lower.starts_with(prefix) {
			let rest = trimmed[prefix.len()..].trim_start();
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
	let title_link = card.select_first("h3 a, h2 a")?;
	let href = title_link.attr("href")?;
	let key = slug_from_href(&href)?;
	let title = title_link
		.text()
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())?;
	let cover = card
		.select_first("img")
		.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
		.filter(|s| !s.is_empty())
		.map(|s| abs_url(&s));
	let status = card
		.select_first(".cover-status, small.eyebrow")
		.and_then(|el| el.text())
		.map(|t| parse_status(&t))
		.unwrap_or(MangaStatus::Unknown);
	let genres: Vec<String> = card
		.select(".genre-badges a, .cherri-detail__chips a")
		.map(|els| {
			els.filter_map(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
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

fn parse_card_chapter(card: &Element) -> Option<MangaWithChapter> {
	let manga = parse_card(card)?;
	let link = card.select_first("a.latest-chapter, a.cherri-list-card__action")?;
	let href = link.attr("href")?;
	let chapter_id = chapter_id_from_href(&href)?;
	let label = link
		.select_first("span")
		.and_then(|el| el.text())
		.or_else(|| link.text())
		.map(|t| clean(&t.replace('→', "")))
		.unwrap_or_default();
	let (chapter_number, title) = parse_chapter_label(&label);
	let date_uploaded = card
		.select_first("time")
		.and_then(|el| el.attr("datetime"))
		.and_then(|dt| parse_iso_utc(&dt));
	Some(MangaWithChapter {
		manga,
		chapter: Chapter {
			url: Some(format!("{BASE_URL}/read/{chapter_id}")),
			key: chapter_id,
			title,
			chapter_number,
			date_uploaded,
			..Default::default()
		},
	})
}

fn home_section(document: &Document, heading: &str) -> Option<Element> {
	document
		.select(".cherri-section-heading")
		.and_then(|headings| {
			headings.into_iter().find_map(|header| {
				let matches = header
					.select_first("h2")
					.and_then(|el| el.text())
					.is_some_and(|text| clean(&text).eq_ignore_ascii_case(heading));
				matches.then(|| header.next()).flatten()
			})
		})
}

fn home_section_manga(document: &Document, heading: &str) -> Vec<Manga> {
	let mut seen: Vec<String> = Vec::new();
	home_section(document, heading)
		.and_then(|section| section.select("article.manga-card"))
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

fn home_section_latest(document: &Document, heading: &str) -> Vec<MangaWithChapter> {
	let mut seen: Vec<String> = Vec::new();
	home_section(document, heading)
		.and_then(|section| section.select("article.manga-card"))
		.map(|cards| {
			cards
				.filter_map(|card| {
					let item = parse_card_chapter(&card)?;
					if seen.contains(&item.manga.key) {
						return None;
					}
					seen.push(item.manga.key.clone());
					Some(item)
				})
				.collect()
		})
		.unwrap_or_default()
}

fn parse_catalogue(document: &Document) -> MangaPageResult {
	let mut seen: Vec<String> = Vec::new();
	let entries = document
		.select("article.cherri-list-card, article.manga-card")
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
		.unwrap_or_default();
	let has_next_page = document.select("nav.pager a").is_some_and(|links| {
		links.into_iter().any(|link| {
			link.text()
				.is_some_and(|text| text.to_ascii_lowercase().contains("next"))
		})
	});
	MangaPageResult {
		entries,
		has_next_page,
	}
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

struct MangaCherri;

impl Source for MangaCherri {
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
			format!("{BASE_URL}/manga")
		} else {
			format!("{BASE_URL}/manga?{qs}")
		};
		Ok(parse_catalogue(&fetch(&url)?))
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
				.select_first(".cherri-detail h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = document
				.select_first(".cherri-detail__cover img")
				.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
				.filter(|s| !s.is_empty())
				.map(|s| abs_url(&s))
				.or_else(|| manga.cover.take());
			manga.description = document
				.select_first(".cherri-detail__description")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			manga.authors = document
				.select_first(
					".cherri-detail__copy > p > strong, .cherri-detail__description + p strong",
				)
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.map(|a| vec![a]);

			let mut genres: Vec<String> = Vec::new();
			if let Some(els) = document.select(".cherri-detail__chips a") {
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
				.select_first(".cherri-detail__copy > p.eyebrow, .cherri-detail p.eyebrow")
				.and_then(|el| el.text())
				.map(|t| parse_status(t.split('·').next_back().unwrap_or(&t)))
				.unwrap_or(MangaStatus::Unknown);
			manga.url = Some(url);
		}

		if needs_chapters {
			let mut seen: Vec<String> = Vec::new();
			let chapters = document
				.select("[data-chapter-list] a[data-chapter-row], a.cherri-chapter-row")
				.map(|rows| {
					rows.filter_map(|row| {
						let href = row.attr("href")?;
						let id = chapter_id_from_href(&href)?;
						if seen.contains(&id) {
							return None;
						}
						seen.push(id.clone());
						let raw_name = row
							.select_first("strong")
							.and_then(|el| el.own_text().or_else(|| el.text()))
							.map(|t| clean(&t))
							.unwrap_or_default();
						let (chapter_number, title) = parse_chapter_label(&raw_name);
						let date_uploaded = row
							.select_first("time")
							.and_then(|el| el.attr("datetime"))
							.and_then(|dt| parse_iso_utc(&dt));
						Some(Chapter {
							url: Some(format!("{BASE_URL}/read/{id}")),
							key: id,
							title,
							chapter_number,
							date_uploaded,
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

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/read/{}", chapter.key);
		let document = fetch(&url)?;
		let pages = document
			.select("[data-cherri-reader] img[data-page], #chapter-reader img[data-page]")
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

impl ListingProvider for MangaCherri {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		match listing.id.as_str() {
			"unfolding" => {
				qs.set("status", Some("ongoing"));
				qs.set("sort", Some("updated"));
			}
			"completed" => {
				qs.set("status", Some("completed"));
				qs.set("sort", Some("updated"));
			}
			"popular" | "added" | "updated" => qs.set("sort", Some(&listing.id)),
			_ => qs.set("sort", Some("updated")),
		}
		if page > 1 {
			qs.set("page", Some(&page.to_string()));
		}
		Ok(parse_catalogue(&fetch(&format!("{BASE_URL}/manga?{qs}"))?))
	}
}

impl Home for MangaCherri {
	fn get_home(&self) -> Result<HomeLayout> {
		let home = fetch(&format!("{BASE_URL}/home"))?;
		let mut components: Vec<HomeComponent> = Vec::new();

		let most_viewed = home_section_manga(&home, "Most viewed manga");
		if !most_viewed.is_empty() {
			components.push(HomeComponent {
				title: Some("Most Viewed".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: most_viewed,
					auto_scroll_interval: Some(6.0),
				},
			});
		}

		let latest = home_section_latest(&home, "New chapter blooms");
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Updates".into()),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: Some(make_listing("updated", "Latest Updates")),
				},
			});
		}

		let unfolding: Vec<Link> = home_section_manga(&home, "Stories still unfolding")
			.into_iter()
			.map(Into::into)
			.collect();
		if !unfolding.is_empty() {
			components.push(HomeComponent {
				title: Some("Stories Still Unfolding".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: unfolding,
					listing: Some(make_listing("unfolding", "Stories Still Unfolding")),
				},
			});
		}

		let completed: Vec<Link> = home_section_manga(&home, "Happily ever after")
			.into_iter()
			.map(Into::into)
			.collect();
		if !completed.is_empty() {
			components.push(HomeComponent {
				title: Some("Completed".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: completed,
					listing: Some(make_listing("completed", "Completed")),
				},
			});
		}

		let recent: Vec<Link> = home_section_manga(&home, "First encounters")
			.into_iter()
			.map(Into::into)
			.collect();
		if !recent.is_empty() {
			components.push(HomeComponent {
				title: Some("Recently Added".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: recent,
					listing: Some(make_listing("added", "Recently Added")),
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

impl ImageRequestProvider for MangaCherri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaCherri {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if let Some(key) = slug_from_href(&url) {
			return Ok(Some(DeepLinkResult::Manga { key }));
		}
		Ok(None)
	}
}

register_source!(
	MangaCherri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
