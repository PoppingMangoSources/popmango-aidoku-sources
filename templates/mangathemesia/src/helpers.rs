use crate::Params;
use aidoku::{
	HomeComponent, HomeComponentValue, Link, LinkValue, Manga,
	alloc::{format, string::String, vec::Vec},
	helpers::string::StripPrefixOrSelf,
	imports::html::{Document, Element},
};

/// The image banners some child themes put at the top of the home page, one series each.
pub fn slider_banners(html: &Document, params: &Params) -> Option<HomeComponent> {
	let links: Vec<Link> = html
		.select(".slidernew .swiper-slide a")?
		.filter_map(|link| {
			let image = link.select_first("img")?;
			Some(Link {
				title: image.attr("alt")?,
				image_url: image.img_attr(),
				value: Some(LinkValue::Manga(Manga {
					key: link
						.attr("href")?
						.strip_prefix_or_self(&params.base_url)
						.into(),
					..Default::default()
				})),
				..Default::default()
			})
		})
		.collect();
	if links.is_empty() {
		return None;
	}
	Some(HomeComponent {
		title: None,
		subtitle: None,
		value: HomeComponentValue::ImageScroller {
			links,
			auto_scroll_interval: Some(5.0),
			width: params.banner_width,
			height: params.banner_height,
		},
	})
}

/// A novel chapter's text as Markdown: each innermost block of `#readerarea` is a paragraph.
pub fn novel_text(html: &Document) -> String {
	html.select("div#readerarea p, div#readerarea div")
		.map(|blocks| {
			blocks
				.filter(|block| {
					!block
						.children()
						.any(|child| matches!(child.tag_name().as_deref(), Some("p" | "div")))
				})
				.filter_map(|block| block.text())
				.filter(|text| !text.trim().is_empty())
				.map(|text| escape_markdown_syntax(text.trim()))
				.collect::<Vec<_>>()
				.join("\n\n")
		})
		.unwrap_or_default()
}

// Only syntax is escaped: the reader's Markdown renderer overflows its stack past a few hundred
// inline nodes in one paragraph, and every escape is a node.
fn escape_markdown_syntax(line: &str) -> String {
	let mut escaped = String::with_capacity(line.len());
	for c in line.chars() {
		// "-", "#" or "1." at the start would begin a list or a heading.
		let block = (escaped.is_empty() && matches!(c, '#' | '-' | '+' | '='))
			|| (matches!(c, '.' | ')')
				&& !escaped.is_empty()
				&& escaped.bytes().all(|b| b.is_ascii_digit()));
		if block
			|| matches!(
				c,
				'\\' | '`' | '*' | '_' | '[' | ']' | '<' | '>' | '&' | '~' | '|'
			) {
			escaped.push('\\');
		}
		escaped.push(c);
	}
	escaped
}

pub trait ElementImageAttr {
	fn img_attr(&self) -> Option<String>;
}

impl ElementImageAttr for Element {
	fn img_attr(&self) -> Option<String> {
		self.attr("abs:data-lazy-src")
			.or_else(|| self.attr("abs:data-src"))
			.or_else(|| self.attr("abs:src"))
	}
}

pub fn extract_images(content: &str) -> Vec<String> {
	let slice = format!(
		"[{}]",
		extract_between(content, "\"images\":[", "]").unwrap_or_default()
	);
	serde_json::from_str::<Vec<String>>(&slice).unwrap_or_default()
}

pub fn extract_between<'a>(s: &'a str, start: &str, end: &str) -> Option<&'a str> {
	s.find(start).and_then(|start_idx| {
		let after_start = &s[start_idx + start.len()..];
		after_start.find(end).map(|end_idx| &after_start[..end_idx])
	})
}

pub fn selector(template: &str, values: &[&str]) -> String {
	let mut selectors = Vec::new();
	for value in values {
		// replace all occurrences of {} with the value
		let mut formatted = String::new();
		let mut parts = template.split("{}");
		if let Some(first) = parts.next() {
			formatted.push_str(first);
		}
		for part in parts {
			formatted.push_str(value);
			formatted.push_str(part);
		}
		selectors.push(formatted);
	}
	selectors.join(", ")
}

pub fn find_first_f32(s: &str) -> Option<f32> {
	let mut num = String::new();
	let mut found_digit = false;
	let mut dot_found = false;

	for c in s.chars() {
		if c.is_ascii_digit() {
			num.push(c);
			found_digit = true;
		} else if c == '.' && found_digit && !dot_found {
			num.push(c);
			dot_found = true;
		} else if found_digit {
			break;
		}
	}

	if found_digit {
		num.parse::<f32>().ok()
	} else {
		None
	}
}
