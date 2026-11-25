use aidoku::{
	Chapter, Listing, ListingKind, Manga, MangaWithChapter,
	alloc::{String, Vec},
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		std::{current_date, parse_date, parse_date_with_options},
	},
};
use madara::helpers::{self as madara_helpers, ElementImageAttr};

pub const BASE_URL: &str = "https://cocomic.co";

pub fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
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
		.filter(|url| !url.is_empty())
}

fn style_image_url(style: &str) -> Option<String> {
	let start = style.find("url(")? + 4;
	let rest = &style[start..];
	let end = rest.find(')')?;
	let url = rest[..end].trim().trim_matches(['\'', '"']);
	(!url.is_empty()).then(|| String::from(url))
}

pub fn card_manga(item: &Element) -> Option<Manga> {
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
	Some(Manga {
		key: href.strip_prefix_or_self(BASE_URL).into(),
		title,
		cover: cover_of(item),
		url: Some(href),
		..Default::default()
	})
}

pub fn hero_entries(document: &Document) -> Vec<Manga> {
	let synopses: Vec<(String, String)> = document
		.select(".hero-slide")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let href = slide.select_first(".hero-actions a")?.attr("abs:href")?;
			let synopsis = slide
				.select_first(".hero-synopsis")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			Some((href.strip_prefix_or_self(BASE_URL).into(), synopsis))
		})
		.collect();

	document
		.select(".mobile-hero-slide")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let href = slide.select_first(".mobile-hero-cta")?.attr("abs:href")?;
			let key: String = href.strip_prefix_or_self(BASE_URL).into();
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

pub fn swimlane_latest(document: &Document, heading: &str) -> Vec<MangaWithChapter> {
	swimlane_section(document, heading)
		.and_then(|section| section.select(".mobile-swimlane-card"))
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let manga = card_manga(&card)?;
			let chapter = card.select_first(".swimlane-chap")?;
			let href = chapter.attr("abs:href")?;
			let label = chapter.text().map(|text| clean(&text))?;
			let date_uploaded = card
				.select_first(".swimlane-time")
				.and_then(|el| el.text())
				.and_then(|text| listing_date(&text));
			let chapter_number = madara_helpers::find_first_f32(&label);
			Some(MangaWithChapter {
				manga,
				chapter: Chapter {
					key: href.strip_prefix_or_self(BASE_URL).into(),
					chapter_number,
					date_uploaded,
					title: clean_chapter_title(&label, chapter_number),
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
