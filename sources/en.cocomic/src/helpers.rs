use aidoku::{
	Chapter, Listing, ListingKind, Manga, MangaWithChapter,
	alloc::{String, Vec, format},
	imports::{
		html::{Document, Element},
		std::{current_date, parse_date, parse_date_with_options},
	},
};
use madara::helpers::{self as madara_helpers, ElementImageAttr};

pub const BASE_URL: &str = "https://cocomic.co";
const CDN_URL: &str = "https://cdn2.cocomic.co";
pub const LOCK_SUFFIX: &str = "#lock";

pub fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn abs_url(raw: &str) -> Option<String> {
	let url = raw.trim();
	if url.is_empty() {
		return None;
	}
	let resolved = if url.starts_with("//") {
		format!("https:{url}")
	} else if let Some(rest) = url.strip_prefix("http://") {
		format!("https://{rest}")
	} else if url.starts_with("https://") {
		String::from(url)
	} else if url.starts_with('/') {
		format!("{BASE_URL}{url}")
	} else {
		format!("{BASE_URL}/{url}")
	};
	// Cocomic serves `/www/wwwroot` paths from `cdn2` when absent on the main origin.
	if let Some(rest) = resolved.strip_prefix(&format!("{BASE_URL}/www/wwwroot/")) {
		return Some(format!("{CDN_URL}/{rest}"));
	}
	Some(resolved)
}

pub fn append_style_list(url: &str) -> String {
	if url.contains("style=list") {
		return String::from(url);
	}
	let sep = if url.contains('?') { '&' } else { '?' };
	format!("{url}{sep}style=list")
}

pub fn parse_manga_chapter_path(value: &str) -> Option<(String, Option<String>)> {
	let without_origin = value
		.trim()
		.strip_prefix(BASE_URL)
		.unwrap_or(value.trim())
		.split(['?', '#'])
		.next()?;
	let after = without_origin
		.strip_prefix("/manga/")
		.or_else(|| without_origin.strip_prefix("manga/"))?;
	let mut parts = after.split('/').filter(|s| !s.is_empty());
	let slug = parts.next()?;
	let manga_key = format!("/manga/{slug}/");
	let chapter_key = parts.next().map(|ch| format!("/manga/{slug}/{ch}/"));
	Some((manga_key, chapter_key))
}

fn largest_srcset(srcset: &str) -> Option<&str> {
	srcset
		.split(',')
		.filter_map(|entry| {
			let mut parts = entry.split_whitespace();
			let url = parts.next()?;
			let width = parts
				.next()
				.and_then(|w| w.strip_suffix('w'))
				.and_then(|w| w.parse::<u32>().ok())
				.unwrap_or(0);
			Some((url, width))
		})
		.max_by_key(|(_, width)| *width)
		.map(|(url, _)| url)
}

pub fn reader_image_url(img: &Element) -> Option<String> {
	let from_srcset = img
		.attr("data-srcset")
		.or_else(|| img.attr("srcset"))
		.and_then(|s| largest_srcset(&s).map(String::from));
	img.attr("data-cfsrc")
		.or_else(|| img.attr("data-src"))
		.or_else(|| img.attr("data-lazy-src"))
		.or(from_srcset)
		.or_else(|| img.attr("src"))
		.and_then(|raw| abs_url(&raw))
}

pub fn listing_date(text: &str) -> Option<i64> {
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
		if let Some(idx) = rest.find(|c: char| !c.is_ascii_digit())
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

pub fn clean_chapter_title(raw: &str, number: Option<f32>) -> Option<String> {
	let trimmed = raw.trim();
	if trimmed.is_empty() {
		return None;
	}
	let Some(num) = number else {
		return Some(trimmed.into());
	};
	if trimmed
		.parse::<f32>()
		.ok()
		.is_some_and(|n| (n - num).abs() < f32::EPSILON)
	{
		return None;
	}
	let rest = trimmed
		.strip_prefix("Chapter")
		.or_else(|| trimmed.strip_prefix("CHAPTER"))
		.or_else(|| trimmed.strip_prefix("chapter"))
		.or_else(|| trimmed.strip_prefix("Ch."))
		.or_else(|| trimmed.strip_prefix("ch."))
		.or_else(|| trimmed.strip_prefix("CH."))
		.or_else(|| trimmed.strip_prefix("Ch "))
		.or_else(|| trimmed.strip_prefix("ch "))
		.or_else(|| trimmed.strip_prefix("Episode"))
		.or_else(|| trimmed.strip_prefix("episode"))
		.or_else(|| trimmed.strip_prefix("Ep."))
		.or_else(|| trimmed.strip_prefix("ep."))
		.or_else(|| trimmed.strip_prefix('#'))
		.map(str::trim_start);
	if let Some(after_prefix) = rest {
		let after_num = after_prefix.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
		let num_str = after_prefix
			.strip_suffix(after_num)
			.unwrap_or("")
			.trim_end_matches('.');
		if num_str
			.parse::<f32>()
			.ok()
			.is_some_and(|n| (n - num).abs() < f32::EPSILON)
		{
			let tail = after_num
				.trim_start_matches(|c: char| {
					c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|')
				})
				.trim();
			return (!tail.is_empty()).then(|| tail.into());
		}
	}
	Some(trimmed.into())
}

fn is_chapter_tag(lower: &str) -> bool {
	let rest = lower
		.strip_prefix("chapter")
		.or_else(|| lower.strip_prefix("ch."))
		.or_else(|| lower.strip_prefix("ch "))
		.or_else(|| lower.strip_prefix("ep."))
		.or_else(|| lower.strip_prefix("ep "))
		.map(str::trim_start);
	rest.is_some_and(|r| {
		!r.is_empty() && r.chars().all(|c| c.is_ascii_digit() || c == '.') && r != "."
	})
}

pub fn is_valid_genre(label: &str) -> bool {
	if label.is_empty() || label.len() > 64 {
		return false;
	}
	let lower = label.to_lowercase();
	!is_chapter_tag(&lower)
		&& !lower.contains("publication:")
		&& !lower.contains("bato status:")
		&& !lower.contains("bato upload status:")
		&& !lower.contains("read direction:")
		&& !lower.contains("tr from")
		&& !label
			.chars()
			.any(|c| ('\u{F000}'..='\u{F8FF}').contains(&c))
}

fn cover_of(element: &Element) -> Option<String> {
	element
		.select_first("img")
		.and_then(|image| image.img_attr(false))
		.and_then(|url| abs_url(&url))
}

fn style_image_url(style: &str) -> Option<String> {
	let start = style.find("url(")? + 4;
	let rest = &style[start..];
	let end = rest.find(')')?;
	abs_url(rest[..end].trim().trim_matches(['\'', '"']))
}

pub fn card_manga(item: &Element) -> Option<Manga> {
	let link = item.select_first(
		".asura-card-title, .swimlane-title, .post-title a, h3 a[href*='/manga/'], h4 a[href*='/manga/']",
	)?;
	let href = link.attr("abs:href").or_else(|| link.attr("href"))?;
	let (key, _) = parse_manga_chapter_path(&href)?;
	let title = link
		.text()
		.or_else(|| link.attr("title"))
		.map(|text| clean(&text))
		.filter(|text| !text.is_empty())?;
	Some(Manga {
		url: Some(format!("{BASE_URL}{key}")),
		key,
		title,
		cover: cover_of(item),
		..Default::default()
	})
}

pub fn hero_entries(document: &Document) -> Vec<Manga> {
	let synopses: Vec<(String, String)> = document
		.select(".hero-slide")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let href = slide.select_first(".hero-actions a")?.attr("href")?;
			let (key, _) = parse_manga_chapter_path(&href)?;
			let synopsis = slide
				.select_first(".hero-synopsis")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			Some((key, synopsis))
		})
		.collect();

	document
		.select(".mobile-hero-slide")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let href = slide.select_first(".mobile-hero-cta")?.attr("href")?;
			let (key, _) = parse_manga_chapter_path(&href)?;
			let title = slide
				.select_first(".mobile-hero-title")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			let cover = slide.attr("style").and_then(|s| style_image_url(&s));
			let mut tags: Vec<String> = slide
				.select(".m-genre")
				.into_iter()
				.flatten()
				.filter_map(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| is_valid_genre(t))
				.collect();
			tags.sort_unstable();
			tags.dedup();
			let description = synopses
				.iter()
				.find(|(k, _)| *k == key)
				.map(|(_, s)| s.clone());
			Some(Manga {
				url: Some(format!("{BASE_URL}{key}")),
				key,
				title,
				cover,
				description,
				tags: (!tags.is_empty()).then_some(tags),
				..Default::default()
			})
		})
		.collect()
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

pub fn swimlane_manga(document: &Document, heading: &str) -> Vec<Manga> {
	swimlane_section(document, heading)
		.and_then(|section| section.select(".mobile-swimlane-card"))
		.into_iter()
		.flatten()
		.filter_map(|card| card_manga(&card))
		.collect()
}

pub fn swimlane_latest(
	document: &Document,
	heading: &str,
	show_locked: bool,
) -> Vec<MangaWithChapter> {
	swimlane_section(document, heading)
		.and_then(|section| section.select(".mobile-swimlane-card"))
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let manga = card_manga(&card)?;
			let chapter = card.select_first(".swimlane-chap")?;
			let locked =
				chapter.has_class("premium") || card.select_first(".coin:not(.free)").is_some();
			if locked && !show_locked {
				return None;
			}
			let href = chapter.attr("abs:href").or_else(|| chapter.attr("href"))?;
			// Cards whose `.swimlane-chap` links to the series page instead of a chapter have no reader page.
			let (_, Some(ch_path)) = parse_manga_chapter_path(&href)? else {
				return None;
			};
			let key = if locked {
				format!("{ch_path}{LOCK_SUFFIX}")
			} else {
				ch_path
			};
			let label = chapter.text().map(|text| clean(&text))?;
			let date_uploaded = card
				.select_first(".swimlane-time")
				.and_then(|el| el.text())
				.and_then(|text| listing_date(&text));
			let chapter_number = madara_helpers::find_first_f32(&label);
			Some(MangaWithChapter {
				manga,
				chapter: Chapter {
					key,
					chapter_number,
					date_uploaded,
					title: clean_chapter_title(&label, chapter_number),
					locked,
					..Default::default()
				},
			})
		})
		.collect()
}

pub fn listing(id: &str, name: &str) -> Listing {
	Listing {
		id: String::from(id),
		name: String::from(name),
		kind: ListingKind::Default,
	}
}
