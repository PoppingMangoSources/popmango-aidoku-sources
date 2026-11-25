#![no_std]
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, Filter, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	ListingKind, Manga, MangaPageResult, MangaStatus, MangaWithChapter, MultiSelectFilter, Page,
	PageContent, PageContext, Result, Source, Viewer,
	alloc::{String, Vec, vec},
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		net::Request,
		std::{current_date, parse_date, parse_date_with_options},
	},
	prelude::*,
};
use madara::{
	Impl, LoadMoreStrategy, Madara, Params,
	helpers::{self, ElementImageAttr},
};

const BASE_URL: &str = "https://cocomic.co";

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn listing_date(text: &str) -> Option<i64> {
	let trimmed = text.trim();
	if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("new") {
		return None;
	}
	let lowered = trimmed.to_lowercase();
	if lowered == "just now" || lowered == "today" {
		return Some(current_date());
	}
	if let Some(rest) = lowered.strip_suffix("ago") {
		let rest = rest.trim();
		let split = rest.find(|c: char| !c.is_ascii_digit());
		if let Some(idx) = split
			&& let Ok(amount) = rest[..idx].parse::<i64>()
		{
			let unit = rest[idx..].trim();
			let seconds = match unit {
				"s" => Some(1),
				"m" => Some(60),
				"h" => Some(3_600),
				"d" => Some(86_400),
				"w" => Some(604_800),
				"mo" => Some(2_592_000),
				"y" => Some(31_536_000),
				u if u.starts_with("second") => Some(1),
				u if u.starts_with("min") => Some(60),
				u if u.starts_with("hour") => Some(3_600),
				u if u.starts_with("day") => Some(86_400),
				u if u.starts_with("week") => Some(604_800),
				u if u.starts_with("month") => Some(2_592_000),
				u if u.starts_with("year") => Some(31_536_000),
				_ => None,
			};
			if let Some(seconds) = seconds {
				return Some(current_date() - amount * seconds);
			}
		}
	}

	parse_date_with_options(trimmed, "MMMM d, yyyy", "en_US_POSIX", "current")
		.or_else(|| parse_date_with_options(trimmed, "MMM d, yyyy", "en_US_POSIX", "current"))
		.or_else(|| parse_date(trimmed, "yyyy-MM-dd"))
}

fn cover_of(element: &Element) -> Option<String> {
	element
		.select_first("img")
		.and_then(|image| image.img_attr(false))
		.filter(|url| !url.is_empty())
}

fn style_image_url(style: &str) -> Option<String> {
	let start = style.find("url(")? + 4;
	let rest = &style[start..];
	let end = rest.find(')')?;
	let url = rest[..end].trim().trim_matches(['\'', '"']);
	(!url.is_empty()).then(|| String::from(url))
}

fn card_manga(item: &Element) -> Option<Manga> {
	let link = item.select_first(
		".asura-card-title, .swimlane-title, .post-title a, h3 a[href*='/manga/'], h4 a[href*='/manga/']",
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
	let cover = cover_of(item);
	Some(Manga {
		key: href.strip_prefix_or_self(BASE_URL).into(),
		title,
		cover,
		url: Some(href),
		..Default::default()
	})
}

fn hero_entries(document: &Document) -> Vec<Manga> {
	let synopses: Vec<(String, String)> = document
		.select(".hero-slide")
		.map(|slides| {
			slides
				.filter_map(|slide| {
					let href = slide.select_first(".hero-actions a")?.attr("abs:href")?;
					let synopsis = slide
						.select_first(".hero-synopsis")
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					Some((href.strip_prefix_or_self(BASE_URL).into(), synopsis))
				})
				.collect()
		})
		.unwrap_or_default();

	document
		.select(".mobile-hero-slide")
		.map(|slides| {
			slides
				.filter_map(|slide| {
					let href = slide.select_first(".mobile-hero-cta")?.attr("abs:href")?;
					let key: String = href.strip_prefix_or_self(BASE_URL).into();
					let title = slide
						.select_first(".mobile-hero-title")
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					let cover = slide.attr("style").and_then(|s| style_image_url(&s));
					let tags: Vec<String> = slide
						.select(".m-genre")
						.map(|els| {
							els.filter_map(|el| el.text())
								.map(|t| clean(&t))
								.filter(|t| !t.is_empty())
								.collect()
						})
						.unwrap_or_default();
					let description = synopses
						.iter()
						.find(|(k, _)| *k == key)
						.map(|(_, s)| s.clone());
					Some(Manga {
						key,
						title,
						cover,
						description,
						tags: (!tags.is_empty()).then_some(tags),
						url: Some(href),
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

fn swimlane_section(document: &Document, heading: &str) -> Option<Element> {
	document
		.select(".mobile-swimlane-section")?
		.find(|section| {
			section
				.select_first(".swimlane-heading")
				.and_then(|el| el.text())
				.is_some_and(|text| clean(&text).eq_ignore_ascii_case(heading))
		})
}

fn swimlane_manga(document: &Document, heading: &str) -> Vec<Manga> {
	swimlane_section(document, heading)
		.and_then(|section| section.select(".mobile-swimlane-card"))
		.map(|cards| cards.filter_map(|card| card_manga(&card)).collect())
		.unwrap_or_default()
}

fn swimlane_latest(document: &Document, heading: &str) -> Vec<MangaWithChapter> {
	swimlane_section(document, heading)
		.and_then(|section| section.select(".mobile-swimlane-card"))
		.map(|cards| {
			cards
				.filter_map(|card| {
					let manga = card_manga(&card)?;
					let chapter = card.select_first(".swimlane-chap")?;
					let href = chapter.attr("abs:href")?;
					let label = chapter.text().map(|text| clean(&text))?;
					let date_uploaded = card
						.select_first(".swimlane-time")
						.and_then(|el| el.text())
						.and_then(|text| listing_date(&text));
					let chapter_number = helpers::find_first_f32(&label);
					Some(MangaWithChapter {
						manga,
						chapter: Chapter {
							key: href.strip_prefix_or_self(BASE_URL).into(),
							chapter_number,
							date_uploaded,
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

fn is_valid_genre(label: &str) -> bool {
	if label.is_empty() || label.len() > 64 {
		return false;
	}
	let lower = label.to_lowercase();
	!lower.contains("publication:")
		&& !lower.contains("bato status:")
		&& !lower.contains("bato upload status:")
		&& !lower.contains("read direction:")
		&& !lower.contains("tr from")
		&& !label
			.chars()
			.any(|c| ('\u{F000}'..='\u{F8FF}').contains(&c))
}

fn make_listing(id: &str, name: &str) -> Listing {
	Listing {
		id: String::from(id),
		name: String::from(name),
		kind: ListingKind::Default,
	}
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
			use_load_more_request: LoadMoreStrategy::Never,
			default_viewer: Viewer::Webtoon,
			genre_endpoint: "/?s=&post_type=wp-manga".into(),
			details_title_selector: "h1.asura-series-title".into(),
			details_cover_selector: "img.asura-poster-img".into(),
			details_author_selector: ".asura-chip-author .author-name-text".into(),
			details_artist_selector: ".artist-content > a".into(),
			details_description_selector: ".asura-synopsis-content".into(),
			details_tag_selector: "a.asura-genre-tag".into(),
			details_status_selector: ".asura-badge-status".into(),
			details_type_selector: ".asura-hero-badge-type".into(),
			chapter_selector: "li.wp-manga-chapter:not(.premium)".into(),
			..Default::default()
		}
	}

	fn get_manga_status(&self, str: &str) -> MangaStatus {
		match str.trim().to_ascii_lowercase().as_str() {
			"ongoing" | "updating" => MangaStatus::Ongoing,
			"completed" => MangaStatus::Completed,
			"on hold" | "hiatus" => MangaStatus::Hiatus,
			"canceled" | "cancelled" | "dropped" => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		}
	}

	fn get_manga_content_rating(&self, _html: &Document, manga: &Manga) -> ContentRating {
		let Some(ref tags) = manga.tags else {
			return ContentRating::Safe;
		};
		let mut suggestive = false;
		for tag in tags {
			match tag.to_ascii_lowercase().as_str() {
				"18" | "18+" | "adult" | "hentai" | "pornographic" | "smut" => {
					return ContentRating::NSFW;
				}
				"ecchi" | "mature" | "soft yaoi" | "soft yuri" | "yaoi" | "yaoibl" | "yuri" => {
					suggestive = true;
				}
				_ => {}
			}
		}
		if suggestive {
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		}
	}

	fn get_dynamic_filters(&self, params: &Params) -> Result<Vec<Filter>> {
		let html = Request::get(format!("{}{}", params.base_url, params.genre_endpoint))?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let mut pairs: Vec<(String, String)> = Vec::new();
		if let Some(inputs) = html.select("input[name]") {
			for input in inputs {
				if input.attr("name").as_deref() != Some("genre[]") {
					continue;
				}
				let Some(id) = input.attr("value") else {
					continue;
				};
				let Some(input_id) = input.attr("id") else {
					continue;
				};
				let Some(label) = html
					.select_first(format!("label[for='{input_id}']"))
					.and_then(|el| el.text())
					.map(|t| clean(&t))
					.map(|t| String::from(t.trim_end_matches([',', ' '])))
				else {
					continue;
				};
				if id.is_empty()
					|| !is_valid_genre(&label)
					|| pairs.iter().any(|(seen_opt, seen_id)| {
						seen_id == &id || seen_opt.eq_ignore_ascii_case(&label)
					}) {
					continue;
				}
				pairs.push((label, id));
			}
		}
		pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));
		let (options, ids): (Vec<_>, Vec<_>) = pairs
			.into_iter()
			.map(|(opt, id)| (opt.into(), id.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genre[]".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: false,
				uses_tag_style: true,
				options,
				ids: Some(ids),
				..Default::default()
			}
			.into(),
		])
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let mut context = PageContext::new();
		context.insert("Referer".into(), url);
		Ok(html
			.select(
				".asura-stream-images img, .reading-content .page-break img, .reading-content img.wp-manga-chapter-img",
			)
			.map(|imgs| {
				imgs.filter_map(|img| {
					let src = img.img_attr(false)?;
					(!src.is_empty()).then(|| Page {
						content: PageContent::url_context(src, context.clone()),
						..Default::default()
					})
				})
				.collect()
			})
			.unwrap_or_default())
	}

	fn get_manga_list(
		&self,
		_params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let order = match listing.id.as_str() {
			"trending" | "latest" | "views" | "new-manga" | "rating" => listing.id.as_str(),
			_ => "latest",
		};
		let url = if page <= 1 {
			format!("{BASE_URL}/manga/?m_orderby={order}")
		} else {
			format!("{BASE_URL}/manga/page/{page}/?m_orderby={order}")
		};
		let html = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let entries = html
			.select("article.asura-card, .c-tabs-item__content")
			.map(|cards| cards.filter_map(|card| card_manga(&card)).collect())
			.unwrap_or_default();
		let has_next_page = html
			.select_first(".asura-pagination a.next, a.next.page-numbers, a.nextpostslink")
			.is_some();
		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}

	fn get_home(&self, _params: &Params) -> Result<HomeLayout> {
		let home = Request::get(format!("{BASE_URL}/"))?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;

		let mut components: Vec<HomeComponent> = Vec::new();

		let featured = hero_entries(&home);
		if !featured.is_empty() {
			components.push(HomeComponent {
				title: Some("Trending".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: featured,
					auto_scroll_interval: Some(6.0),
				},
			});
		}

		let trending_now: Vec<Link> = swimlane_manga(&home, "Trending Now")
			.into_iter()
			.map(Into::into)
			.collect();
		if !trending_now.is_empty() {
			components.push(HomeComponent {
				title: Some("Trending Now".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: trending_now,
					listing: Some(make_listing("trending", "Trending Now")),
				},
			});
		}

		let latest = swimlane_latest(&home, "Latest Updates");
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Updates".into()),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: Some(make_listing("latest", "Latest Updates")),
				},
			});
		}

		let popular: Vec<Link> = swimlane_manga(&home, "Popular Manga")
			.into_iter()
			.map(Into::into)
			.collect();
		if !popular.is_empty() {
			components.push(HomeComponent {
				title: Some("Popular Manga".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: popular,
					listing: Some(make_listing("views", "Popular Manga")),
				},
			});
		}

		let new_releases: Vec<Link> = swimlane_manga(&home, "New Releases")
			.into_iter()
			.map(Into::into)
			.collect();
		if !new_releases.is_empty() {
			components.push(HomeComponent {
				title: Some("New Releases".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: new_releases,
					listing: Some(make_listing("new-manga", "New Releases")),
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
	DynamicFilters,
	DeepLinkHandler,
	ImageRequestProvider
);
