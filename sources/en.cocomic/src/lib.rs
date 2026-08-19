#![no_std]
use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, MangaWithChapter, Result,
	Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		net::Request,
		std::{current_date, parse_date},
	},
	prelude::*,
};
use madara::{Impl, Madara, Params};

const BASE_URL: &str = "https://cocomic.co";

/// The rows below the banner: a title, the browse url that fills it, and whether
/// it renders as a ranked list.
fn home_rows() -> [(&'static str, String, bool); 5] {
	[
		("New Releases", browse_url("new-manga"), false),
		("Trending", browse_url("trending"), true),
		("Most Viewed", browse_url("views"), true),
		("Yaoi", genre_url("yaoi"), false),
		("Manhwa", genre_url("manhwa"), false),
	]
}

fn browse_url(order: &str) -> String {
	format!("{BASE_URL}/manga/?m_orderby={order}")
}

fn genre_url(slug: &str) -> String {
	format!("{BASE_URL}/?s=&post_type=wp-manga&genre%5B%5D={slug}&m_orderby=views")
}

/// Resolves the relative or absolute stamps the listing cards carry.
fn listing_date(text: &str) -> Option<i64> {
	let trimmed = text.trim();
	if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("new") {
		return None;
	}

	let lowered = trimmed.to_lowercase();
	let mut words = lowered.trim_end_matches("ago").split_whitespace();
	if let Some(amount) = words.next().and_then(|word| word.parse::<i64>().ok())
		&& let Some(unit) = words.next()
	{
		let seconds = if unit.starts_with("second") {
			Some(1)
		} else if unit.starts_with("min") {
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

	parse_date(trimmed, "MMMM d, yyyy")
		.or_else(|| parse_date(trimmed, "MMM d, yyyy"))
		.or_else(|| parse_date(trimmed, "yyyy-MM-dd"))
}

fn cover_of(element: &Element) -> Option<String> {
	let image = element.select_first(".item-thumb img, .tab-thumb img, img")?;
	image
		.attr("abs:data-src")
		.or_else(|| image.attr("abs:data-lazy-src"))
		.or_else(|| image.attr("abs:srcset"))
		.or_else(|| image.attr("abs:src"))
}

fn chapter_number(title: &str) -> Option<f32> {
	let mut number = String::new();
	for ch in title.chars() {
		if ch.is_ascii_digit() || (ch == '.' && !number.is_empty()) {
			number.push(ch);
		} else if !number.is_empty() {
			break;
		}
	}
	number.trim_matches('.').parse().ok()
}

fn card_manga(item: &Element) -> Option<Manga> {
	let link = item.select_first(".post-title a, h3 a, a")?;
	let href = link.attr("abs:href")?;
	let title = link
		.text()
		.or_else(|| link.attr("title"))
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty())?;
	Some(Manga {
		key: href.strip_prefix_or_self(BASE_URL).into(),
		title,
		cover: cover_of(item),
		url: Some(href),
		..Default::default()
	})
}

fn browse_cards(document: &Document) -> Vec<Manga> {
	document
		.select(".page-item-detail, .c-tabs-item__content")
		.map(|items| items.filter_map(|item| card_manga(&item)).collect())
		.unwrap_or_default()
}

struct Cocomic;

impl Impl for Cocomic {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			use_new_chapter_endpoint: true,
			..Default::default()
		}
	}

	fn get_home(&self, _params: &Params) -> Result<HomeLayout> {
		// The banner and every shelf is its own browse query, so they all go out
		// together.
		let rows = home_rows();
		let mut urls = vec![BASE_URL.to_string(), browse_url("rating")];
		urls.extend(rows.iter().map(|(_, url, _)| url.clone()));
		let requests = urls
			.iter()
			.map(|url| Request::get(url).map_err(Into::into))
			.collect::<Result<Vec<_>>>()?;
		let documents: Vec<Option<Document>> = Request::send_all(requests)
			.into_iter()
			.map(|response| response.ok().and_then(|response| response.get_html().ok()))
			.collect();

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Some(document) = documents.get(1).and_then(Option::as_ref) {
			let entries: Vec<Manga> = browse_cards(document).into_iter().take(10).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Top Rated".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries,
						auto_scroll_interval: Some(6.0),
					},
				});
			}
		}

		// The home page carries the dated update list.
		if let Some(home) = documents.first().and_then(Option::as_ref) {
			let latest: Vec<MangaWithChapter> = home
				.select(".page-item-detail")
				.map(|items| {
					items
						.filter_map(|item| {
							let manga = card_manga(&item)?;
							let chapter = item.select_first(
								".latest-chap .chapter a, .chapter-item .chapter a",
							)?;
							let href = chapter.attr("abs:href").or_else(|| chapter.attr("href"))?;
							let label = chapter.text().map(|text| text.trim().to_string())?;
							let date = item
								.select_first(".chapter-release-date, .post-on")
								.and_then(|el| {
									el.select_first(".c-new-tag")
										.and_then(|tag| tag.attr("title"))
										.or_else(|| el.text())
								})
								.and_then(|text| listing_date(text.trim()));
							Some(MangaWithChapter {
								manga,
								chapter: Chapter {
									key: href.strip_prefix_or_self(BASE_URL).into(),
									chapter_number: chapter_number(&label),
									date_uploaded: date,
									title: (!label.is_empty()).then_some(label),
									url: Some(href),
									..Default::default()
								},
							})
						})
						.collect()
				})
				.unwrap_or_default();
			if !latest.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Updates".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: None,
						entries: latest,
						listing: None,
					},
				});
			}
		}

		for ((title, _, ranked), document) in rows.iter().zip(documents.into_iter().skip(2)) {
			let Some(document) = document else { continue };
			let entries: Vec<Link> = browse_cards(&document)
				.into_iter()
				.map(Into::into)
				.collect();
			if entries.is_empty() {
				continue;
			}
			components.push(HomeComponent {
				title: Some((*title).into()),
				subtitle: None,
				value: if *ranked {
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

register_source!(
	Madara<Cocomic>,
	Home,
	ListingProvider,
	DeepLinkHandler,
	ImageRequestProvider
);
