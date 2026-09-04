#![no_std]
use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, Link, Listing, ListingProvider, Manga, MangaPageResult,
	MangaStatus, MangaWithChapter, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, string::ToString},
	helpers::uri::encode_uri_component,
	imports::{
		html::{Document, Element},
		net::Request,
		std::{current_date, parse_date},
	},
	prelude::*,
};

const BASE_URL: &str = "https://www.mangahome.com";

const MATURE_GENRES: &[&str] = &["ecchi", "mature", "smut", "yaoi", "yuri", "adult", "harem"];

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn abs_url(url: &str) -> String {
	if url.is_empty() {
		String::new()
	} else if url.starts_with("http") {
		url.to_string()
	} else if let Some(rest) = url.strip_prefix("//") {
		format!("https://{rest}")
	} else if url.starts_with('/') {
		format!("{BASE_URL}{url}")
	} else {
		format!("{BASE_URL}/{url}")
	}
}

fn img_from(element: &Element) -> Option<String> {
	let src = element
		.attr("data-src")
		.or_else(|| element.attr("data-lazy-src"))
		.or_else(|| element.attr("data-cfsrc"))
		.or_else(|| element.attr("src"))
		.unwrap_or_default();
	(!src.is_empty()).then(|| abs_url(&src))
}

/// The manga id is the segment after `/manga/`.
fn manga_id_from(href: &str) -> Option<String> {
	let after = href.split("/manga/").nth(1)?;
	let id = after.split(['/', '?', '#']).next()?;
	(!id.is_empty()).then(|| id.to_string())
}

/// Chapter links are `/manga/<slug>/(v../)?c../`; the id is everything after
/// the slug.
fn chapter_ref_from(href: &str) -> Option<(String, String)> {
	let after = href.split("/manga/").nth(1)?;
	let mut segments = after.splitn(2, '/');
	let slug = segments.next()?.to_string();
	let rest = segments.next()?;
	let chapter = rest
		.split(['?', '#'])
		.next()
		.unwrap_or("")
		.trim_matches('/');
	(!slug.is_empty() && !chapter.is_empty()).then(|| (slug, chapter.to_string()))
}

fn chapter_number(text: &str) -> Option<f32> {
	let lower = text.to_lowercase();
	let start = lower.find('c').map(|i| i + 1).filter(|_| {
		// Prefer the `c<number>` marker; fall back to first digit run.
		lower.contains('c')
	});
	let slice = match start {
		Some(i) if lower[i..].starts_with(|c: char| c.is_ascii_digit()) => &lower[i..],
		_ => lower.as_str(),
	};
	let mut number = String::new();
	for ch in slice.chars() {
		if ch.is_ascii_digit() || (ch == '.' && !number.is_empty()) {
			number.push(ch);
		} else if !number.is_empty() {
			break;
		}
	}
	number.trim_matches('.').parse().ok()
}

fn content_rating_for(genres: &[String]) -> ContentRating {
	let lowered: Vec<String> = genres.iter().map(|g| g.trim().to_lowercase()).collect();
	if lowered.iter().any(|g| MATURE_GENRES.contains(&g.as_str())) {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

fn parse_site_date(text: &str) -> Option<i64> {
	let lowered = text.trim().to_lowercase();
	if lowered.is_empty() {
		return None;
	}
	if lowered.contains("today") {
		return Some(current_date());
	}
	if lowered.contains("yesterday") {
		return Some(current_date() - 86400);
	}
	let mut words = lowered.trim_end_matches("ago").split_whitespace();
	if let Some(amount) = words.next().and_then(|w| w.parse::<i64>().ok())
		&& let Some(unit) = words.next()
	{
		let seconds = if unit.starts_with("min") {
			Some(60)
		} else if unit.starts_with("hour") {
			Some(3600)
		} else if unit.starts_with("day") {
			Some(86400)
		} else if unit.starts_with("week") {
			Some(604800)
		} else if unit.starts_with("month") {
			Some(2592000)
		} else if unit.starts_with("year") {
			Some(31536000)
		} else {
			None
		};
		if let Some(seconds) = seconds {
			return Some(current_date() - amount * seconds);
		}
	}
	parse_date(text.trim(), "MMM d, yyyy").or_else(|| parse_date(text.trim(), "MMM d,yyyy"))
}

struct ListItem {
	manga: Manga,
	chapter: Option<Chapter>,
}

fn parse_list(document: &Document) -> Vec<ListItem> {
	let Some(items) = document.select("ul.manga-list > li") else {
		return Vec::new();
	};
	let mut out = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	for item in items {
		let cover = item.select_first("a.post-cover");
		let title_link = item.select_first("p.title a");
		let href = title_link
			.as_ref()
			.and_then(|a| a.attr("href"))
			.or_else(|| cover.as_ref().and_then(|c| c.attr("href")))
			.unwrap_or_default();
		let Some(id) = manga_id_from(&href) else {
			continue;
		};
		if seen.contains(&id) {
			continue;
		}
		let title = title_link
			.as_ref()
			.and_then(|a| a.attr("title").or_else(|| a.text()))
			.or_else(|| cover.as_ref().and_then(|c| c.attr("title")))
			.map(|t| clean(&t))
			.filter(|t| !t.is_empty());
		let Some(title) = title else { continue };
		seen.push(id.clone());

		let genres: Vec<String> = item
			.select("p.genre a")
			.map(|els| {
				els.filter_map(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
					.collect()
			})
			.unwrap_or_default();
		let cover_img = cover
			.as_ref()
			.and_then(|c| c.select_first("img"))
			.and_then(|img| img_from(&img));

		let chapter = item
			.select_first("a[href*='/manga/'][href*='c']")
			.and_then(|a| {
				let chref = a.attr("href")?;
				let (_, cid) = chapter_ref_from(&chref)?;
				let label = a.attr("title").or_else(|| a.text()).map(|t| clean(&t))?;
				let date = item
					.select_first("p.time")
					.and_then(|el| el.text())
					.and_then(|t| parse_site_date(&t));
				Some(Chapter {
					key: cid.clone(),
					chapter_number: chapter_number(&label).or_else(|| chapter_number(&cid)),
					date_uploaded: date,
					title: (!label.is_empty()).then_some(label),
					url: Some(format!("{BASE_URL}/manga/{id}/{cid}/")),
					..Default::default()
				})
			});

		out.push(ListItem {
			manga: Manga {
				key: id.clone(),
				title,
				cover: cover_img,
				content_rating: content_rating_for(&genres),
				tags: (!genres.is_empty()).then_some(genres),
				url: Some(format!("{BASE_URL}/manga/{id}/")),
				..Default::default()
			},
			chapter,
		});
	}
	out
}

fn fetch(url: &str) -> Result<Document> {
	Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()
		.map_err(Into::into)
}

fn directory_url(path: &str, sort: &str) -> String {
	let token = match sort {
		"rating" => "rating.za",
		_ => "",
	};
	if token.is_empty() {
		format!("{BASE_URL}/{path}")
	} else {
		format!("{BASE_URL}/{path}?{token}")
	}
}

struct MangaHome;

impl Source for MangaHome {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.unwrap_or_default();
		let query = query.trim();

		let mut status = String::new();
		let mut included: Vec<String> = Vec::new();
		let mut excluded: Vec<String> = Vec::new();
		for filter in &filters {
			match filter {
				FilterValue::Select { id, value } if id == "status" && !value.is_empty() => {
					status = value.clone();
				}
				FilterValue::MultiSelect {
					id,
					included: inc,
					excluded: exc,
				} if id == "genres" => {
					included.extend(inc.iter().cloned());
					excluded.extend(exc.iter().cloned());
				}
				_ => {}
			}
		}
		let has_filters = !status.is_empty() || !included.is_empty() || !excluded.is_empty();

		let url = if query.is_empty() && !has_filters {
			if page > 1 {
				format!("{BASE_URL}/latest/{page}.html")
			} else {
				format!("{BASE_URL}/latest")
			}
		} else {
			// The advanced-search endpoint accepts an optional name plus genre and
			// status filters, gated behind advopts=1.
			let mut params: Vec<String> = Vec::new();
			if !query.is_empty() {
				params.push(format!("name={}", encode_uri_component(query)));
				params.push("name_method=cw".into());
			}
			if !included.is_empty() {
				params.push(format!(
					"ingenres={}",
					encode_uri_component(included.join(","))
				));
			}
			if !excluded.is_empty() {
				params.push(format!(
					"exgenres={}",
					encode_uri_component(excluded.join(","))
				));
			}
			if !status.is_empty() {
				params.push(format!("is_completed={status}"));
			}
			params.push("advopts=1".into());
			if page > 1 {
				params.push(format!("page={page}"));
			}
			format!("{BASE_URL}/search?{}", params.join("&"))
		};
		let document = fetch(&url)?;
		let entries: Vec<Manga> = parse_list(&document).into_iter().map(|i| i.manga).collect();
		Ok(MangaPageResult {
			has_next_page: !entries.is_empty()
				&& document.select_first("a.next, a:contains(Next)").is_some(),
			entries,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let document = fetch(&format!("{BASE_URL}/manga/{}/", manga.key))?;
		if needs_details {
			if let Some(title) = document
				.select_first("div.detail-info h1, div.manga-detail h1")
				.and_then(|el| el.own_text().or_else(|| el.text()))
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = document
				.select_first("img.detail-cover")
				.and_then(|img| img_from(&img))
				.or_else(|| manga.cover.take());
			manga.authors = document.select("a[href*='/author/']").map(|els| {
				els.filter_map(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
					.collect()
			});
			manga.artists = document.select("a[href*='/artist/']").map(|els| {
				els.filter_map(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
					.collect()
			});
			let genres: Vec<String> = document
				.select("div.manga-detailmiddle p a[href*='/directory/'], p.detail-genre a")
				.map(|els| {
					els.filter_map(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())
						.collect()
				})
				.unwrap_or_default();
			manga.description = document
				.select_first("p.detail-info-right-say, div.manga-detailmiddle p.hide")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			let status_text = document
				.select("div.manga-detailmiddle p")
				.map(|els| {
					els.filter_map(|el| el.text())
						.find(|t| t.to_lowercase().contains("status"))
						.unwrap_or_default()
				})
				.unwrap_or_default()
				.to_lowercase();
			manga.status = if status_text.contains("complet") {
				MangaStatus::Completed
			} else if status_text.contains("ongoing") {
				MangaStatus::Ongoing
			} else {
				MangaStatus::Unknown
			};
			manga.content_rating = content_rating_for(&genres);
			manga.tags = (!genres.is_empty()).then_some(genres);
			manga.url = Some(format!("{BASE_URL}/manga/{}/", manga.key));
		}
		if needs_chapters {
			manga.chapters = document
				.select("ul.detail-chlist > li a[href]")
				.map(|links| {
					let items: Vec<Element> = links.collect();
					let total = items.len();
					items
						.into_iter()
						.enumerate()
						.filter_map(|(index, a)| {
							let href = a.attr("href")?;
							let (_, cid) = chapter_ref_from(&href)?;
							let label = a
								.select_first("span.vol")
								.and_then(|el| el.text())
								.or_else(|| a.text())
								.map(|t| clean(&t))
								.unwrap_or_default();
							let date = a
								.select_first("span.time")
								.and_then(|el| el.text())
								.and_then(|t| parse_site_date(&t));
							Some(Chapter {
								chapter_number: chapter_number(&label)
									.or_else(|| chapter_number(&cid))
									.or(Some((total - index) as f32)),
								date_uploaded: date,
								title: (!label.is_empty()).then_some(label),
								url: Some(format!("{BASE_URL}/manga/{}/{cid}/", manga.key)),
								key: cid,
								..Default::default()
							})
						})
						.collect()
				});
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let document = fetch(&format!("{BASE_URL}/manga/{}/{}/", manga.key, chapter.key))?;
		let mut seen: Vec<String> = Vec::new();
		let pages = document
			.select("#viewer img, section.mangaread-img img, .read-img img")
			.map(|images| {
				images
					.filter_map(|img| {
						let url = img_from(&img)?;
						(!seen.contains(&url)).then(|| {
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

impl Home for MangaHome {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			format!("{BASE_URL}/latest"),
			directory_url("shoujo", "views"),
			directory_url("shoujo", "rating"),
			directory_url("yaoi", "views"),
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

		let latest_doc = documents.next().flatten();
		let shoujo_views = documents.next().flatten();
		let shoujo_rating = documents.next().flatten();
		let yaoi_views = documents.next().flatten();

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Some(document) = shoujo_views.as_ref() {
			let entries: Vec<Manga> = parse_list(document)
				.into_iter()
				.take(10)
				.map(|i| i.manga)
				.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Top Viewed Shoujo".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries,
						auto_scroll_interval: Some(6.0),
					},
				});
			}
		}

		if let Some(document) = latest_doc.as_ref() {
			let entries: Vec<MangaWithChapter> = parse_list(document)
				.into_iter()
				.filter_map(|item| {
					let chapter = item.chapter?;
					Some(MangaWithChapter {
						manga: item.manga,
						chapter,
					})
				})
				.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Releases".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: Some(5),
						entries,
						listing: None,
					},
				});
			}
		}

		for (title, document, ranked) in [
			("Top Rated Shoujo", shoujo_rating, true),
			("Most Viewed Yaoi", yaoi_views, false),
		] {
			let Some(document) = document else { continue };
			let entries: Vec<Link> = parse_list(&document)
				.into_iter()
				.map(|i| i.manga.into())
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
						page_size: Some(5),
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

impl ListingProvider for MangaHome {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let url = match listing.id.as_str() {
			"latest" if page > 1 => format!("{BASE_URL}/latest/{page}.html"),
			"latest" => format!("{BASE_URL}/latest"),
			path if page > 1 => format!("{BASE_URL}/{path}/{page}.html"),
			path => format!("{BASE_URL}/{path}"),
		};
		let document = fetch(&url)?;
		let entries: Vec<Manga> = parse_list(&document).into_iter().map(|i| i.manga).collect();
		Ok(MangaPageResult {
			has_next_page: !entries.is_empty(),
			entries,
		})
	}
}

impl aidoku::ImageRequestProvider for MangaHome {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaHome {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		if let Some((slug, chapter)) = chapter_ref_from(&url) {
			return Ok(Some(DeepLinkResult::Chapter {
				manga_key: slug,
				key: chapter,
			}));
		}
		let Some(id) = manga_id_from(&url) else {
			return Ok(None);
		};
		Ok(Some(DeepLinkResult::Manga { key: id }))
	}
}

register_source!(
	MangaHome,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
