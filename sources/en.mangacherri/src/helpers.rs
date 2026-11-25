use aidoku::{
	Chapter, ContentRating, Listing, ListingKind, Manga, MangaPageResult, MangaStatus,
	MangaWithChapter, Result, Viewer,
	alloc::{String, Vec, collections::BTreeSet, format, string::ToString},
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
};

pub const BASE_URL: &str = "https://mangacherri.com";
pub const SORTS: [&str; 6] = [
	"updated",
	"added",
	"popular",
	"chapters",
	"title",
	"title_desc",
];
const MATURE_GENRES: &[&str] = &["seinen", "ecchi", "harem", "mature", "adult", "smut"];

pub fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn abs_url(url: &str) -> String {
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

pub fn slug_from_href(href: &str) -> Option<String> {
	let after = href.split("/title/").nth(1)?;
	let slug = after.split(['/', '?', '#']).next()?;
	(!slug.is_empty()).then(|| slug.into())
}

pub fn chapter_id_from_href(href: &str) -> Option<String> {
	let after = href.split("/read/").nth(1)?;
	let id = after.split(['/', '?', '#']).next()?;
	(!id.is_empty() && id.bytes().all(|b| b.is_ascii_digit())).then(|| id.into())
}

pub fn parse_status(value: &str) -> MangaStatus {
	match value.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("complet") => MangaStatus::Completed,
		s if s.contains("hiatus") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("drop") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

pub fn content_rating_for(genres: &[String]) -> ContentRating {
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

pub fn viewer_for(genres: &[String]) -> Viewer {
	if genres.iter().any(|g| {
		let lower = g.to_ascii_lowercase();
		lower.contains("manhwa") || lower.contains("manhua") || lower.contains("webtoon")
	}) {
		Viewer::Webtoon
	} else {
		Viewer::RightToLeft
	}
}

pub fn parse_iso_utc(raw: &str) -> Option<i64> {
	let trimmed = raw.trim();
	parse_date_with_options(
		trimmed,
		"yyyy-MM-dd'T'HH:mm:ss.SSSXXX",
		"en_US_POSIX",
		"UTC",
	)
	.or_else(|| parse_date_with_options(trimmed, "yyyy-MM-dd'T'HH:mm:ssXXX", "en_US_POSIX", "UTC"))
	.or_else(|| {
		let naive = trimmed
			.split('.')
			.next()?
			.trim_end_matches("+00:00")
			.trim_end_matches('Z');
		parse_date_with_options(naive, "yyyy-MM-ddTHH:mm:ss", "en_US_POSIX", "UTC")
	})
	.or_else(|| parse_date_with_options(trimmed, "yyyy-MM-dd", "en_US_POSIX", "UTC"))
}

pub fn parse_chapter_label(name: &str) -> (Option<f32>, Option<String>) {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return (None, None);
	}
	if let Ok(num) = trimmed.parse::<f32>() {
		return (Some(num), None);
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
		if let Ok(num) = num_str.parse::<f32>() {
			let tail = after_num
				.trim_start_matches(|c: char| {
					c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|')
				})
				.trim();
			return (Some(num), (!tail.is_empty()).then(|| tail.into()));
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
	(num, Some(trimmed.into()))
}

fn parse_card(card: &Element) -> Option<Manga> {
	let title_link = card.select_first("h3 a, h2 a")?;
	let key = slug_from_href(&title_link.attr("href")?)?;
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
		.into_iter()
		.flatten()
		.filter_map(|el| el.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())
		.collect();

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
	let chapter_id = chapter_id_from_href(&link.attr("href")?)?;
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
		.select(".cherri-section-heading")?
		.into_iter()
		.find_map(|header| {
			let matches = header
				.select_first("h2")
				.and_then(|el| el.text())
				.is_some_and(|text| clean(&text).eq_ignore_ascii_case(heading));
			matches.then(|| header.next()).flatten()
		})
}

pub fn home_section_manga(document: &Document, heading: &str) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	home_section(document, heading)
		.and_then(|section| section.select("article.manga-card"))
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let manga = parse_card(&card)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect()
}

pub fn home_section_latest(document: &Document, heading: &str) -> Vec<MangaWithChapter> {
	let mut seen = BTreeSet::new();
	home_section(document, heading)
		.and_then(|section| section.select("article.manga-card"))
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let item = parse_card_chapter(&card)?;
			seen.insert(item.manga.key.clone()).then_some(item)
		})
		.collect()
}

pub fn parse_catalogue(document: &Document) -> MangaPageResult {
	let mut seen = BTreeSet::new();
	let entries = document
		.select("article.cherri-list-card, article.manga-card")
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let manga = parse_card(&card)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect();
	let has_next_page = document
		.select("nav.pager a")
		.into_iter()
		.flatten()
		.any(|link| {
			link.text()
				.is_some_and(|text| text.to_ascii_lowercase().contains("next"))
		});
	MangaPageResult {
		entries,
		has_next_page,
	}
}

pub fn fetch(url: &str) -> Result<Document> {
	Ok(Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()?)
}

pub fn listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}
