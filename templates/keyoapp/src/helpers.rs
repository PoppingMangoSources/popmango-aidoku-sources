use aidoku::{
	Chapter, Manga, MangaStatus, MangaWithChapter,
	alloc::{String, string::ToString},
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		std::{current_date, parse_date_with_options},
	},
	prelude::*,
};
use base64::{Engine, engine::general_purpose::STANDARD};

/// A card's series: the link holds the path and title, the cover is a background image.
pub fn manga(link: &Element, cover: Option<Element>, base_url: &str) -> Option<Manga> {
	Some(Manga {
		key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: link
			.attr("title")
			.or_else(|| link.attr("alt"))?
			.trim()
			.into(),
		cover: cover.and_then(|cover| background_image(&cover)),
		..Default::default()
	})
}

/// The url in an element's `background-image: url(...)` or `--photoURL: url(...)` style.
pub fn background_image(element: &Element) -> Option<String> {
	let style = element.attr("style")?;
	let url = style.split_once("url(")?.1.split_once(')')?.0;
	Some(url.trim_matches(['\'', '"', ' ']).into())
}

// The template has no reader for novels, which the site lists among its comics.
pub fn is_novel(card: &Element) -> bool {
	card.attr("data-type")
		.is_some_and(|kind| kind.eq_ignore_ascii_case("novel"))
		|| has_badge(card, "novel")
}

/// Whether a card carries a badge with this text, such as "novel" or "Upcoming".
pub fn has_badge(card: &Element, badge: &str) -> bool {
	card.select("span").is_some_and(|mut spans| {
		spans.any(|span| {
			span.own_text()
				.is_some_and(|text| text.trim().eq_ignore_ascii_case(badge))
		})
	})
}

/// The text beside a details page label such as "Status" or "Author": each sits in a block with
/// its value right after it. Selectors only, since the test runner can't walk to a sibling.
pub fn labelled(html: &Document, label: &str) -> Option<String> {
	html.select("div.grid")?.find_map(|block| {
		let name = block.select_first("div.font-medium > span")?.text()?;
		if name.trim() != label {
			return None;
		}
		block
			.select_first("div.font-medium + div")?
			.text()
			.map(|text| text.trim().to_string())
			.filter(|text| !text.is_empty())
	})
}

pub fn status(text: &str) -> MangaStatus {
	match text.to_lowercase().as_str() {
		"ongoing" => MangaStatus::Ongoing,
		"completed" => MangaStatus::Completed,
		"dropped" => MangaStatus::Cancelled,
		"hiatus" | "paused" => MangaStatus::Hiatus,
		_ => MangaStatus::Unknown,
	}
}

/// "Chapter 16 - A Mighty Enemy!" as its number and the name after it.
pub fn chapter_parts(label: &str) -> (Option<f32>, Option<String>) {
	let label = label.trim();
	let Some(rest) = label.strip_prefix("Chapter") else {
		return (None, Some(label.into()));
	};
	let rest = rest.trim_start();
	let end = rest
		.find(|c: char| !(c.is_ascii_digit() || c == '.'))
		.unwrap_or(rest.len());
	let name = rest[end..].trim_start_matches([' ', '-', ':']).trim();
	(
		rest[..end].parse().ok(),
		(!name.is_empty()).then(|| name.into()),
	)
}

/// A chapter date: "6 hours ago" for recent uploads, "Aug 3, 2026" for older ones.
pub fn chapter_date(text: &str) -> Option<i64> {
	let text = text.trim();
	if !text.ends_with("ago") {
		return parse_date_with_options(text, "MMM d, yyyy", "en_US_POSIX", "current");
	}
	let amount = text.split_whitespace().next().and_then(|word| match word {
		"a" | "an" | "one" => Some(1),
		number => number.parse::<i64>().ok(),
	})?;
	let unit = if text.contains("sec") {
		1
	} else if text.contains("min") {
		60
	} else if text.contains("hour") {
		60 * 60
	} else if text.contains("day") {
		24 * 60 * 60
	} else if text.contains("week") {
		7 * 24 * 60 * 60
	} else if text.contains("month") {
		30 * 24 * 60 * 60
	} else if text.contains("year") {
		365 * 24 * 60 * 60
	} else {
		return None;
	};
	Some(current_date() - amount * unit)
}

// A heading sits a level or two inside the grid that holds its cards. A heading with no cards of
// its own stops at the first block that holds another row's heading.
pub fn section(heading: &Element) -> Option<Element> {
	let title = heading.text();
	let mut node = heading.parent();
	for _ in 0..3 {
		let element = node?;
		if element
			.select("h2")
			.is_some_and(|mut headings| headings.any(|other| other.text() != title))
		{
			return None;
		}
		if element
			.select_first(".latest-poster, .editor-pick-button, button[title] a[href*='/series/']")
			.is_some()
		{
			return Some(element);
		}
		node = element.parent();
	}
	None
}

// A pick has no link; the site's script opens `/series/<data-id>`, shows the cover from the
// upload host and decodes the description from base64.
pub fn editor_pick(pick: &Element) -> Option<Manga> {
	let description = pick
		.attr("data-description")
		.and_then(|encoded| STANDARD.decode(encoded).ok())
		.and_then(|bytes| String::from_utf8(bytes).ok())
		.map(|text| text.trim().into())
		.filter(|text: &String| !text.is_empty());
	Some(Manga {
		key: format!("/series/{}/", pick.attr("data-id")?),
		title: pick.attr("title")?.trim().into(),
		cover: pick
			.attr("data-img")
			.map(|image| format!("https://wsrv.nl/?w=600&url=cdn.meowing.org/uploads/{image}")),
		description,
		..Default::default()
	})
}

pub fn chapter(link: &Element, base_url: &str) -> Option<Chapter> {
	let key: String = link.attr("href")?.strip_prefix_or_self(base_url).into();
	let (chapter_number, title) = chapter_parts(&link.attr("title")?);
	Some(Chapter {
		url: Some(format!("{base_url}{key}")),
		key,
		title,
		chapter_number,
		date_uploaded: link.attr("d").and_then(|date| chapter_date(&date)),
		..Default::default()
	})
}

// A latest row lists each series' newest chapters; coin chapters need a login.
pub fn newest_chapter(poster: &Element, base_url: &str) -> Option<MangaWithChapter> {
	let chapter = poster
		.select("a[href*='/chapter/']")?
		.filter(|link| link.select_first("img[alt=Coin]").is_none())
		.find_map(|link| chapter(&link, base_url))?;
	Some(MangaWithChapter {
		manga: manga(
			&poster.select_first("a[href*='/series/']")?,
			poster.select_first("[style*=background-image]"),
			base_url,
		)?,
		chapter,
	})
}
