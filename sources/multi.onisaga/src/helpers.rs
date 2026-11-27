use crate::api::{BASE_URL, CDN_URL};
use aidoku::{
	ContentRating, MangaStatus, Viewer,
	alloc::{String, Vec},
	imports::std::{parse_date, parse_date_with_options},
};

pub const GENRES: [&str; 31] = [
	"action",
	"adventure",
	"boys-love",
	"comedy",
	"cooking",
	"crime",
	"drama",
	"fantasy",
	"gender-bender",
	"girls-love",
	"harem",
	"historical",
	"horror",
	"isekai",
	"magical-girls",
	"martial-arts",
	"mecha",
	"medical",
	"music",
	"mystery",
	"philosophical",
	"psychological",
	"romance",
	"science-fiction",
	"slice-of-life",
	"sports",
	"superhero",
	"supernatural",
	"thriller",
	"tragedy",
	"wuxia",
];

pub const DEMOGRAPHICS: [&str; 5] = ["shounen", "shoujo", "seinen", "josei", "kodomo"];

pub const FORMS: [&str; 5] = ["serial", "oneshot", "anthology", "doujinshi", "artbook"];

#[derive(Debug, PartialEq)]
pub enum Link {
	Title(String),
	Chapter(String, String),
}

pub fn parse_link(url: &str) -> Option<Link> {
	let path = url.trim().strip_prefix(BASE_URL)?;
	let mut segments = path
		.split(['?', '#'])
		.next()?
		.split('/')
		.filter(|segment| !segment.is_empty());
	match (
		segments.next(),
		segments.next(),
		segments.next(),
		segments.next(),
	) {
		(Some("title"), Some(slug), None, None) => Some(Link::Title(slug.into())),
		(Some("read"), Some(slug), Some(release), None) if release != "continue" => {
			Some(Link::Chapter(slug.into(), release.into()))
		}
		_ => None,
	}
}

pub fn parse_status(value: &str) -> MangaStatus {
	match value.to_ascii_lowercase().as_str() {
		"releasing" => MangaStatus::Ongoing,
		"completed" => MangaStatus::Completed,
		"hiatus" => MangaStatus::Hiatus,
		"cancelled" => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

pub fn parse_content_rating(value: Option<&str>) -> ContentRating {
	match value.map(str::to_ascii_lowercase).as_deref() {
		Some("safe") => ContentRating::Safe,
		Some("suggestive") => ContentRating::Suggestive,
		Some("mature" | "explicit" | "erotica" | "pornographic") => ContentRating::NSFW,
		_ => ContentRating::Unknown,
	}
}

pub fn viewer(layout: Option<&str>, origin: Option<&str>, tags: &[String]) -> Viewer {
	match layout {
		Some("vertical_strip") => Viewer::Webtoon,
		Some("paged_rtl" | "yonkoma") => Viewer::RightToLeft,
		Some("paged_ltr") => Viewer::LeftToRight,
		_ if matches!(origin, Some("manhwa" | "manhua"))
			|| tags.iter().any(|tag| {
				tag.eq_ignore_ascii_case("Long Strip") || tag.eq_ignore_ascii_case("Web Comic")
			}) =>
		{
			Viewer::Webtoon
		}
		_ => match origin {
			Some("manga") => Viewer::RightToLeft,
			Some("comic" | "oel") => Viewer::LeftToRight,
			_ => Viewer::Unknown,
		},
	}
}

pub fn demographic_name(slug: &str) -> Option<&'static str> {
	match slug {
		"shounen" => Some("Shounen"),
		"shoujo" => Some("Shoujo"),
		"seinen" => Some("Seinen"),
		"josei" => Some("Josei"),
		"kodomo" => Some("Kodomo"),
		_ => None,
	}
}

fn strip_ascii_prefix<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
	let (head, tail) = text.split_at_checked(prefix.len())?;
	head.eq_ignore_ascii_case(prefix).then_some(tail)
}

fn strip_chapter_prefix(text: &str) -> Option<(&str, bool)> {
	let mut rest = text.trim();
	if let Some((_, after)) = rest.split_once('·') {
		rest = after.trim();
	}
	if strip_ascii_prefix(rest, "vol").is_some() {
		return None;
	}
	for prefix in ["chapter", "chap", "ch", "episode", "ep", "side story"] {
		if let Some(tail) = strip_ascii_prefix(rest, prefix) {
			let tail = tail.strip_prefix('.').unwrap_or(tail);
			if tail.is_empty() || tail.starts_with(char::is_whitespace) {
				return Some((tail.trim(), true));
			}
		}
	}
	Some((rest, false))
}

pub fn non_empty(value: String) -> Option<String> {
	let trimmed = value.trim();
	if trimmed.is_empty() {
		return None;
	}
	Some(if trimmed.len() == value.len() {
		value
	} else {
		trimmed.into()
	})
}

pub fn parse_chapter_number(label: &str) -> Option<f32> {
	strip_chapter_prefix(label)?.0.parse().ok()
}

pub fn clean_chapter_title(
	title: Option<String>,
	label: &str,
	number: Option<f32>,
) -> Option<String> {
	if let Some(title) = title.and_then(non_empty)
		&& !title.eq_ignore_ascii_case(label.trim())
	{
		let mut rest = title.as_str();
		for vol in ["volume", "vol"] {
			if let Some(after) = strip_ascii_prefix(rest, vol) {
				let after = after.trim_start_matches('.').trim_start();
				let non_num = after.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
				if non_num.len() < after.len() {
					rest = non_num
						.trim_start_matches([' ', '-', ':', ',', '·'])
						.trim_start();
				}
				break;
			}
		}
		let duplicate = strip_chapter_prefix(rest).is_some_and(|(tail, prefixed)| {
			tail.parse::<f32>()
				.ok()
				.is_some_and(|parsed| prefixed || Some(parsed) == number)
		});
		if !duplicate {
			return Some(title);
		}
	}
	if number.is_none() {
		let label = label.trim();
		if !label.is_empty() {
			return Some(label.into());
		}
	}
	None
}

fn fold_ascii(ch: char, out: &mut String) {
	if ch.is_ascii() {
		out.push(ch.to_ascii_lowercase());
		return;
	}
	let folded = match ch {
		'à' | 'á' | 'â' | 'ã' | 'ä' | 'å' | 'ā' | 'À' | 'Á' | 'Â' | 'Ã' | 'Ä' | 'Å' | 'Ā' => {
			"a"
		}
		'æ' | 'Æ' => "ae",
		'ç' | 'ć' | 'č' | 'Ç' | 'Ć' | 'Č' => "c",
		'è' | 'é' | 'ê' | 'ë' | 'ē' | 'È' | 'É' | 'Ê' | 'Ë' | 'Ē' => "e",
		'ì' | 'í' | 'î' | 'ï' | 'ī' | 'Ì' | 'Í' | 'Î' | 'Ï' | 'Ī' => "i",
		'ñ' | 'ń' | 'Ñ' | 'Ń' => "n",
		'ò' | 'ó' | 'ô' | 'õ' | 'ö' | 'ø' | 'ō' | 'Ò' | 'Ó' | 'Ô' | 'Õ' | 'Ö' | 'Ø' | 'Ō' => {
			"o"
		}
		'œ' | 'Œ' => "oe",
		'ß' => "ss",
		'ù' | 'ú' | 'û' | 'ü' | 'ū' | 'Ù' | 'Ú' | 'Û' | 'Ü' | 'Ū' => "u",
		'ý' | 'ÿ' | 'Ý' | 'Ÿ' => "y",
		'\'' | '\u{2019}' => return,
		_ => " ",
	};
	out.push_str(folded);
}

// OniSaga term slugs accept only `[a-z0-9]+(?:-[a-z0-9]+)*`.
pub fn slugify(name: &str) -> String {
	let mut ascii = String::with_capacity(name.len());
	for ch in name.chars() {
		fold_ascii(ch, &mut ascii);
	}
	let mut slug = String::new();
	for word in ascii
		.split(|c: char| !c.is_ascii_alphanumeric())
		.filter(|word| !word.is_empty())
	{
		if !slug.is_empty() {
			slug.push('-');
		}
		slug.push_str(word);
	}
	slug
}

pub fn parse_terms(input: &str, include: &mut Vec<String>, exclude: &mut Vec<String>) {
	for raw in input.split(',') {
		let (target, name) = match raw.trim().strip_prefix(['-', '−']) {
			Some(name) => (&mut *exclude, name),
			None => (&mut *include, raw),
		};
		let slug = slugify(name);
		if !slug.is_empty() {
			target.push(slug);
		}
	}
}

fn strip_markdown_line(line: &str) -> String {
	let line = line
		.trim()
		.trim_start_matches('#')
		.trim_start_matches('>')
		.trim();
	let mut out = String::with_capacity(line.len());
	let mut chars = line.chars().peekable();
	while let Some(ch) = chars.next() {
		match ch {
			'\\' => {
				if let Some(escaped) = chars.next() {
					out.push(escaped);
				}
			}
			'[' => {
				let mut label = String::new();
				let mut closed = false;
				for next in chars.by_ref() {
					if next == ']' {
						closed = true;
						break;
					}
					label.push(next);
				}
				if closed && chars.peek() == Some(&'(') {
					chars.next();
					for next in chars.by_ref() {
						if next == ')' {
							break;
						}
					}
					out.push_str(&label);
				} else {
					out.push('[');
					out.push_str(&label);
					if closed {
						out.push(']');
					}
				}
			}
			'*' | '_' => {}
			_ => out.push(ch),
		}
	}
	out
}

pub fn plain_synopsis(text: &str) -> Option<String> {
	let mut joined = String::new();
	for raw in text.lines() {
		let cleaned = strip_markdown_line(raw);
		let trimmed = cleaned.trim();
		if trimmed.is_empty() {
			continue;
		}
		let lower = trimmed.to_ascii_lowercase();
		if lower.starts_with("(source:")
			|| lower.starts_with("[source:")
			|| lower.starts_with("source:")
		{
			continue;
		}
		if !joined.is_empty() {
			joined.push(' ');
		}
		joined.push_str(trimmed);
	}
	if let Some((before, tail)) = joined.rsplit_once('(')
		&& let Some(inner) = tail.strip_suffix(')')
		&& strip_ascii_prefix(inner.trim_start(), "source:").is_some()
	{
		joined.truncate(before.trim_end().len());
	}
	(!joined.is_empty()).then_some(joined)
}

// Safe pages carry `p=sg` and load from the CDN; gated pages need the main host's `__Host-` cookie.
pub fn media_url(url: String) -> String {
	let safe = url
		.split_once('?')
		.is_some_and(|(_, query)| query.split('&').any(|part| part == "p=sg"));
	if safe {
		url
	} else {
		url.replacen(CDN_URL, BASE_URL, 1)
	}
}

pub fn page_fragment(url: &str) -> Option<(&str, &str, i32)> {
	let (clean, fragment) = url.split_once("#oni=")?;
	let (release_id, ord) = fragment.split_once('.')?;
	Some((clean, release_id, ord.parse().ok()?))
}

pub fn page_expired(url: &str, now: i64) -> bool {
	let Some((_, query)) = url.split_once('?') else {
		return false;
	};
	query
		.split(['&', '#'])
		.find_map(|part| part.strip_prefix("e="))
		.and_then(|value| value.parse::<i64>().ok())
		.is_some_and(|expiry| now + 60 >= expiry)
}

pub fn parse_iso_date(time: &str) -> Option<i64> {
	parse_date_with_options(
		time,
		"yyyy-MM-dd'T'HH:mm:ss.SSSSSSXXX",
		"en_US_POSIX",
		"UTC",
	)
	.or_else(|| parse_date_with_options(time, "yyyy-MM-dd'T'HH:mm:ss.SSSXXX", "en_US_POSIX", "UTC"))
	.or_else(|| parse_date_with_options(time, "yyyy-MM-dd'T'HH:mm:ssXXX", "en_US_POSIX", "UTC"))
	.or_else(|| {
		// Fallback for hosts whose formatter drops fractional seconds and zones.
		let naive = time.split('.').next()?;
		let naive = naive.trim_end_matches("+00:00").trim_end_matches('Z');
		parse_date(naive, "yyyy-MM-ddTHH:mm:ss")
	})
}
