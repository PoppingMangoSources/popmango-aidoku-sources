use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, MangaWithChapter, Viewer,
	alloc::{String, Vec, string::ToString},
	helpers::string::{PlainText, StripPrefixOrSelf},
	imports::{
		html::{Document, Element, Kind},
		std::{current_date, parse_date_with_options},
	},
	prelude::*,
};
use base64::{Engine, engine::general_purpose::STANDARD};

pub fn manga(link: &Element, cover: Option<Element>, base_url: &str) -> Option<Manga> {
	Some(Manga {
		key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: link.attr("title")?.trim().into(),
		cover: cover.and_then(|cover| background_image(&cover)),
		..Default::default()
	})
}

// Covers sit in a `background-image: url(...)` or `--photoURL: url(...)` style
pub fn background_image(element: &Element) -> Option<String> {
	let style = element.attr("style")?;
	let url = style.split_once("url(")?.1.split_once(')')?.0;
	Some(url.trim_matches(['\'', '"', ' ']).into())
}

// A details label such as "Status" sits in a block with its value right after it; selectors only,
// since the test runner can't walk to a sibling
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

pub fn names(text: &str) -> Vec<String> {
	text.split(", ").map(|name| name.trim().into()).collect()
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

// "Chapter 16 - A Mighty Enemy!" as its number and the name after it
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

// "6 hours ago" (or "6h ago" on the second generation) for the last week's uploads, "Aug 3, 2026"
// for older ones
pub fn chapter_date(text: &str) -> Option<i64> {
	let text = text.trim();
	let Some(ago) = text.strip_suffix("ago") else {
		return parse_date_with_options(text, "MMM d, yyyy", "en_US_POSIX", "current");
	};
	let ago = ago.trim();
	let digits = ago.find(|c: char| !c.is_ascii_digit()).unwrap_or(ago.len());
	let amount: i64 = ago[..digits].parse().ok()?;
	let unit = ago[digits..].trim();
	let seconds = if unit == "m" || unit.starts_with("min") {
		60
	} else if unit.starts_with('h') {
		60 * 60
	} else if unit.starts_with('d') {
		24 * 60 * 60
	} else {
		return None;
	};
	Some(current_date() - amount * seconds)
}

pub fn content_rating<'a>(tags: impl Iterator<Item = &'a str> + Clone) -> ContentRating {
	let has_any = |names: &[&str]| {
		tags.clone()
			.any(|tag| names.iter().any(|name| tag.eq_ignore_ascii_case(name)))
	};
	if has_any(&["Adult", "Smut", "Hentai"]) {
		ContentRating::NSFW
	} else if has_any(&["Ecchi", "Mature"]) {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

pub fn viewer(kind: &str) -> Viewer {
	match kind.to_lowercase().as_str() {
		"manga" => Viewer::RightToLeft,
		"comic" => Viewer::LeftToRight,
		// Second generation novels are image strips too
		"manhwa" | "manhua" | "mangatoon" | "novel" => Viewer::Webtoon,
		_ => Viewer::Unknown,
	}
}

// A novel chapter as one Markdown paragraph per `p`: the reader's Markdown overflows its stack on a
// paragraph of hundreds of inline nodes
pub fn novel_text(reader: &Element) -> Option<String> {
	let paragraphs: Vec<String> = reader
		.select("p")?
		.flat_map(|paragraph| {
			let mut text = String::new();
			inline_markdown(&paragraph, &mut text);
			// A `br` splits a paragraph in two
			text.split("\n\n")
				.map(|part| part.trim().to_string())
				.filter(|part| !part.is_empty())
				.collect::<Vec<_>>()
		})
		.collect();
	(!paragraphs.is_empty()).then(|| paragraphs.join("\n\n"))
}

// Text comes from child nodes and tags from children, whose elements the test runner can name
fn inline_markdown(element: &Element, output: &mut String) {
	let mut children = element.children();
	for node in element.child_nodes() {
		match node.kind() {
			Kind::TextNode => {
				if let Some(text) = node.text() {
					output.push_str(&text.escape_markdown());
				}
			}
			Kind::Element => {
				let Some(child) = children.next() else {
					continue;
				};
				let marker = match child.tag_name().as_deref() {
					Some("br") => {
						output.push_str("\n\n");
						continue;
					}
					Some("strong" | "b") => "**",
					Some("em" | "i") => "*",
					_ => "",
				};
				let mut inner = String::new();
				inline_markdown(&child, &mut inner);
				// "** word**" isn't bold, so spaces stay outside the markers
				let trimmed = inner.trim();
				if marker.is_empty() || trimmed.is_empty() {
					output.push_str(&inner);
					continue;
				}
				let start = inner.len() - inner.trim_start().len();
				output.push_str(&inner[..start]);
				output.push_str(marker);
				output.push_str(trimmed);
				output.push_str(marker);
				output.push_str(&inner[start + trimmed.len()..]);
			}
			_ => {}
		}
	}
}

// A heading sits a level or two inside the grid that holds its cards; one with no cards of its own
// stops at the first block holding another row's heading
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

// A pick has no link: the site's script opens `/series/<data-id>`, shows the cover from the
// upload host and decodes the description from base64
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

// A latest row lists each series' newest chapters, and coin chapters need a login
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

// Sites use one of two series layouts: `l2-` classes, or labelled_block blocks with the cover only in
// the page's metadata
pub fn series_card(html: &Document) -> Option<Manga> {
	html.select_first("[data-series-uid]")?;
	let slug = html
		.select_first("[data-series-slug]")?
		.attr("data-series-slug")?;
	Some(Manga {
		key: format!("/{slug}"),
		title: html
			.select_first("h1.l2-title")
			.or_else(|| html.select_first("h1"))?
			.text()?
			.trim()
			.into(),
		cover: html
			.select_first(".l2-poster img")
			.and_then(|image| image.attr("data-src"))
			.or_else(|| {
				html.select_first("meta[property='og:image']")?
					.attr("content")
			}),
		..Default::default()
	})
}

// A labelled_block block's value: "Status" over "Ongoing", or "Author" beside the names
pub fn labelled_block(html: &Document, label: &str) -> Option<String> {
	html.select("div.bg-surface.rounded-lg")?.find_map(|block| {
		let name = block.select_first(".text-xs")?.text()?;
		if name.trim() != label {
			return None;
		}
		block
			.select_first("span.text-sm")?
			.text()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
	})
}

// Posters carry their title in `title`, search and image slider cards in a heading
pub fn card_manga(card: &Element, base_url: &str) -> Option<Manga> {
	Some(Manga {
		key: card.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: card
			.attr("title")
			.or_else(|| card.select_first("h2, h3")?.text())?
			.trim()
			.into(),
		cover: card
			.select_first("img")
			.and_then(|image| image.attr("data-src")),
		..Default::default()
	})
}

// A recently updated card lists its newest chapters, and coin ones need a login
pub fn recent_chapter(card: &Element, base_url: &str) -> Option<MangaWithChapter> {
	let series = card.select_first("h2 a")?;
	let link = card.select("a")?.find(|link| {
		link.select_first("time").is_some() && link.select_first("img[alt$=coins]").is_none()
	})?;
	let key: String = link.attr("href")?.strip_prefix_or_self(base_url).into();
	let (chapter_number, title) = chapter_parts(&link.select_first("span")?.text()?);
	Some(MangaWithChapter {
		manga: Manga {
			key: series.attr("href")?.strip_prefix_or_self(base_url).into(),
			title: series.text()?.trim().into(),
			cover: card
				.select_first("img")
				.and_then(|image| image.attr("data-src")),
			..Default::default()
		},
		chapter: Chapter {
			url: Some(format!("{base_url}{key}")),
			key,
			title,
			chapter_number,
			date_uploaded: link
				.select_first("time")
				.and_then(|time| time.text())
				.and_then(|date| chapter_date(&date)),
			..Default::default()
		},
	})
}
