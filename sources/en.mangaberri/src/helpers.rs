use aidoku::{
	ContentRating, Listing, ListingKind, Manga, MangaStatus, Result, Viewer,
	alloc::{String, Vec, collections::BTreeSet, format, string::ToString},
	imports::{
		html::{Document, Element},
		net::Request,
	},
};

pub const BASE_URL: &str = "https://mangaberri.com";
pub const SORTS: [&str; 3] = ["latest", "popular", "title"];
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

pub fn chapter_from_href(href: &str) -> Option<(String, String)> {
	let after = href.split("/read/").nth(1)?;
	let mut parts = after.split(['/', '?', '#']);
	let slug = parts.next()?;
	let id = parts.next()?;
	(!slug.is_empty() && !id.is_empty() && id.bytes().all(|b| b.is_ascii_digit()))
		.then(|| (slug.into(), id.into()))
}

pub fn parse_status(value: &str) -> MangaStatus {
	match value.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("complet") || s.contains("finish") => MangaStatus::Completed,
		s if s.contains("hiatus") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("discontinu") => MangaStatus::Cancelled,
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

pub fn parse_hero(document: &Document) -> Vec<Manga> {
	document
		.select("section.hero")
		.into_iter()
		.flatten()
		.filter_map(|hero| {
			let href = hero.select_first("a[href*='/title/']")?.attr("href")?;
			let key = slug_from_href(&href)?;
			let title = hero
				.select_first("h2.featured-title, h2")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			let cover = hero
				.select_first(".hero-art img, img")
				.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
				.filter(|s| !s.is_empty())
				.map(|s| abs_url(&s));
			let description = hero
				.select_first("p.hero-description")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			let status = hero
				.select_first(".hero-tags span.pink")
				.and_then(|el| el.text())
				.map(|t| parse_status(&t))
				.unwrap_or(MangaStatus::Unknown);
			let mut tags: Vec<String> = hero
				.select(".hero-tags span:not(.pink)")
				.into_iter()
				.flatten()
				.filter_map(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.collect();
			tags.sort_unstable();
			tags.dedup();
			Some(Manga {
				url: Some(format!("{BASE_URL}/title/{key}")),
				key,
				title,
				cover,
				description,
				status,
				content_rating: content_rating_for(&tags),
				tags: (!tags.is_empty()).then_some(tags),
				..Default::default()
			})
		})
		.collect()
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
		.into_iter()
		.flatten()
		.filter_map(|el| el.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty() && !t.to_ascii_lowercase().contains("chapter"))
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

pub fn parse_cards_in(document: &Document, selector: &str) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	document
		.select(selector)
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let manga = parse_card(&card)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect()
}

pub fn has_next_page(document: &Document) -> bool {
	document
		.select("nav.pagination a")
		.into_iter()
		.flatten()
		.any(|link| {
			link.text()
				.is_some_and(|text| text.to_ascii_lowercase().contains("next"))
		})
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
