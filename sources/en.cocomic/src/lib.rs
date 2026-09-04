#![no_std]
use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, MangaWithChapter, Result,
	Source,
	alloc::{String, Vec},
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

fn browse_url(order: &str) -> String {
	format!("{BASE_URL}/manga/?m_orderby={order}")
}

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
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
	let image = element.select_first(
		".item-thumb img, .tab-thumb img, .related__thumb img, .slider__thumb img, img",
	)?;
	image
		.attr("abs:data-cfsrc")
		.or_else(|| image.attr("abs:data-src"))
		.or_else(|| image.attr("abs:data-lazy-src"))
		.or_else(|| image.attr("abs:src"))
		.filter(|url| !url.is_empty())
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
	// Only anchors that point at a series title the card; a bare `a` fallback
	// would otherwise latch onto a genre link (e.g. "manhwa").
	let link = item.select_first(
		".post-title a, .related__title a, .slider__content h4 a, h3 a[href*='/manga/'], h4 a[href*='/manga/']",
	)?;
	let href = link.attr("abs:href")?;
	if !href.contains("/manga/") {
		return None;
	}
	let title = link
		.text()
		.or_else(|| link.attr("title"))
		.map(|text| clean(&text))
		.filter(|text| !text.is_empty())?;
	let cover = cover_of(item)?;
	Some(Manga {
		key: href.strip_prefix_or_self(BASE_URL).into(),
		title,
		cover: Some(cover),
		url: Some(href),
		..Default::default()
	})
}

fn browse_cards(document: &Document) -> Vec<Manga> {
	document
		.select(".page-item-detail, .c-tabs-item__content, .related__item, .slider__item")
		.map(|items| items.filter_map(|item| card_manga(&item)).collect())
		.unwrap_or_default()
}

/// A homepage rail, located by its `.tp-heading` text and read from the
/// following sliders block.
fn homepage_rail(document: &Document, title: &str) -> Vec<Manga> {
	let Some(headings) = document.select(".tp-heading") else {
		return Vec::new();
	};
	for heading in headings {
		let own = heading
			.own_text()
			.or_else(|| heading.text())
			.map(|t| clean(&t))
			.unwrap_or_default();
		if !own.eq_ignore_ascii_case(title) {
			continue;
		}
		let mut sibling = heading.next();
		while let Some(el) = sibling {
			if el.has_class("wp-block-wp-manga-gutenberg-manga-sliders-block") {
				return el
					.select(".related__item, .slider__item")
					.map(|items| items.filter_map(|i| card_manga(&i)).collect())
					.unwrap_or_default();
			}
			sibling = el.next();
		}
	}
	Vec::new()
}

/// The dated latest-update rows from the `/new/` listing.
fn latest_updates(document: &Document) -> Vec<MangaWithChapter> {
	document
		.select(".page-item-detail")
		.map(|items| {
			items
				.filter_map(|item| {
					let manga = card_manga(&item)?;
					let chapter =
						item.select_first(".latest-chap .chapter a, .chapter-item .chapter a")?;
					let href = chapter.attr("abs:href").or_else(|| chapter.attr("href"))?;
					let label = chapter.text().map(|text| clean(&text))?;
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
		// Ranked shelves come from browse queries; the curated rails and the dated
		// updates come from the homepage and `/new/`. Fetch them all together.
		let urls = [
			browse_url("rating"),
			browse_url("trending"),
			browse_url("views"),
			format!("{BASE_URL}/new/"),
			format!("{BASE_URL}/"),
		];
		let requests = urls
			.iter()
			.map(|url| Request::get(url).map_err(Into::into))
			.collect::<Result<Vec<_>>>()?;
		let documents: Vec<Option<Document>> = Request::send_all(requests)
			.into_iter()
			.map(|response| response.ok().and_then(|response| response.get_html().ok()))
			.collect();
		let doc = |index: usize| documents.get(index).and_then(Option::as_ref);

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Some(document) = doc(0) {
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

		if let Some(document) = doc(1) {
			let entries: Vec<Link> = browse_cards(document).into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Trending".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(5),
						entries,
						listing: None,
					},
				});
			}
		}

		if let Some(document) = doc(3) {
			let latest = latest_updates(document);
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

		if let Some(document) = doc(2) {
			let entries: Vec<Link> = browse_cards(document).into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Viewed".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(5),
						entries,
						listing: None,
					},
				});
			}
		}

		// Curated homepage rails.
		if let Some(home) = doc(4) {
			for title in [
				"Only Cocomic",
				"New Releases",
				"Today's Official",
				"Yaoi",
				"Manhwa",
				"Smut",
			] {
				let entries: Vec<Link> = homepage_rail(home, title)
					.into_iter()
					.map(Into::into)
					.collect();
				if entries.is_empty() {
					continue;
				}
				components.push(HomeComponent {
					title: Some(title.into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries,
						listing: None,
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}

register_source!(Madara<Cocomic>, Home, DeepLinkHandler, ImageRequestProvider);
