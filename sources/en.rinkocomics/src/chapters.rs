use aidoku::{
	Chapter, Result,
	alloc::{String, Vec, format, string::ToString},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element, Html},
		net::Request,
		std::{current_date, parse_date, parse_date_with_options},
	},
	prelude::*,
};
use base64::prelude::*;
use serde::Deserialize;

use crate::{BASE_URL, clean, path_of};

pub const LOCK_SUFFIX: &str = "#lock";
const CHAPTER_SELECTOR: &str = "#cwChapters a.chapter-item, #cwVolumes [data-ep] > a, li.chapter";
const LOAD_MORE_SELECTOR: &str = "#cwMoreChapters, #loadMoreChaptersBtn";
const CHAPTERS_PER_PAGE: usize = 10;
const MAX_CHAPTER_PAGES: usize = 60;

#[derive(Deserialize)]
struct AjaxResponse {
	success: Option<bool>,
	data: Option<AjaxData>,
}

#[derive(Deserialize)]
struct AjaxData {
	html: Option<String>,
}

pub fn parse_rinko_date(text: &str) -> Option<i64> {
	let trimmed = text.split('·').next().unwrap_or(text).trim();
	if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("new") {
		return None;
	}
	let lowered = trimmed.to_ascii_lowercase();
	if lowered == "just now" || lowered == "today" {
		return Some(current_date());
	}
	if lowered == "yesterday" {
		return Some(current_date() - 86_400);
	}
	let relative = lowered.strip_suffix("ago").unwrap_or(&lowered).trim();
	if let Some(split) = relative.find(|c: char| !c.is_ascii_digit())
		&& let Ok(amount) = relative[..split].parse::<i64>()
	{
		let unit = relative[split..].trim();
		let seconds = match unit {
			"s" | "sec" | "secs" | "second" | "seconds" => Some(1),
			"m" | "min" | "mins" | "minute" | "minutes" => Some(60),
			"h" | "hr" | "hrs" | "hour" | "hours" => Some(3_600),
			"d" | "day" | "days" => Some(86_400),
			"w" | "wk" | "wks" | "week" | "weeks" => Some(604_800),
			"mo" | "mos" | "month" | "months" => Some(2_592_000),
			"y" | "yr" | "yrs" | "year" | "years" => Some(31_536_000),
			_ => None,
		};
		if let Some(seconds) = seconds {
			return Some(current_date() - amount * seconds);
		}
	}

	parse_date_with_options(trimmed, "MMMM d, yyyy", "en_US_POSIX", "current")
		.or_else(|| parse_date_with_options(trimmed, "MMM d, yyyy", "en_US_POSIX", "current"))
		.or_else(|| parse_date(trimmed, "yyyy-MM-dd"))
}

pub fn chapter_number_of(name: &str) -> Option<f32> {
	let mut digits = String::new();
	let mut seen_dot = false;
	for ch in name.chars() {
		if ch.is_ascii_digit() {
			digits.push(ch);
		} else if ch == '.' && !seen_dot && !digits.is_empty() {
			seen_dot = true;
			digits.push(ch);
		} else if !digits.is_empty() {
			break;
		}
	}
	digits.trim_end_matches('.').parse().ok()
}

pub fn is_locked_row(row: &Element, href: &str) -> bool {
	if let Some(reason) = row
		.attr("data-reason")
		.map(|r| clean(&r).to_ascii_lowercase())
		&& !reason.is_empty()
		&& reason != "free"
	{
		return true;
	}
	if row.has_class("locked-chapter") || row.has_class("is-locked") {
		return true;
	}
	href.is_empty() || href == "#" || row.select_first(".chapter_price").is_some()
}

fn parse_nonce(body: &str) -> Option<String> {
	let start = body.find("comicworld_ajax")?;
	let rest = &body[start..];
	let brace = rest.find('{')?;
	let end = rest[brace..].find('}')?;
	let block = &rest[brace..brace + end];
	let marker = "\"nonce\"";
	let n_pos = block.find(marker)?;
	let after = block[n_pos + marker.len()..].trim_start();
	let after = after.strip_prefix(':')?.trim_start();
	let after = after.strip_prefix('"')?;
	let close = after.find('"')?;
	Some(after[..close].to_string())
}

pub fn has_more_chapters(doc: &Document) -> bool {
	doc.select_first(LOAD_MORE_SELECTOR)
		.and_then(|el| el.attr("data-comic-id"))
		.is_some_and(|id| !id.trim().is_empty())
}

fn clean_chapter_title(name: &str, number: Option<f32>) -> Option<String> {
	let trimmed = name.trim();
	if trimmed.is_empty() {
		return None;
	}
	if let Some(num) = number {
		let lower = trimmed.to_ascii_lowercase();
		for prefix in ["chapter", "ch.", "ch", "episode", "ep.", "ep", "#"] {
			if let Some(after) = lower.strip_prefix(prefix) {
				let rest = trimmed[trimmed.len() - after.len()..].trim_start();
				let mut end = 0;
				let mut seen_dot = false;
				for (i, ch) in rest.char_indices() {
					if ch.is_ascii_digit() {
						end = i + ch.len_utf8();
					} else if ch == '.' && !seen_dot && end > 0 {
						seen_dot = true;
						end = i + 1;
					} else {
						break;
					}
				}
				let num_str = rest[..end].trim_end_matches('.');
				if num_str
					.parse::<f32>()
					.ok()
					.is_some_and(|n| (n - num).abs() < f32::EPSILON)
				{
					let tail = rest
						.strip_prefix(num_str)
						.unwrap_or("")
						.trim_start_matches(|c: char| {
							c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|')
						})
						.trim();
					return (!tail.is_empty()).then(|| tail.to_string());
				}
			}
		}
	}
	Some(trimmed.to_string())
}

fn parse_rows(doc: &Document, show_locked: bool, out: &mut Vec<Chapter>, seen: &mut Vec<String>) {
	let Some(rows) = doc.select(CHAPTER_SELECTOR) else {
		return;
	};
	for row in rows {
		let href = row
			.attr("data-permalink")
			.or_else(|| row.attr("href"))
			.or_else(|| row.select_first("a").and_then(|a| a.attr("href")))
			.map(|s| clean(&s))
			.unwrap_or_default();
		let post_id = row
			.attr("data-post-id")
			.map(|s| clean(&s))
			.unwrap_or_default();
		if href.is_empty() && post_id.is_empty() {
			continue;
		}
		let locked = is_locked_row(&row, &href);
		if locked && !show_locked {
			continue;
		}
		let path = if !href.is_empty() && href != "#" {
			path_of(&href)
		} else {
			String::new()
		};
		let base_key = if !path.is_empty() {
			path
		} else if !post_id.is_empty() {
			format!("locked-{post_id}")
		} else {
			continue;
		};
		let key = if locked {
			format!("{base_key}{LOCK_SUFFIX}")
		} else {
			base_key.clone()
		};
		if seen.contains(&key) {
			continue;
		}
		seen.push(key.clone());

		let ep_badge = row
			.select_first("span.font-semibold")
			.and_then(|el| el.text())
			.map(|t| clean(&t))
			.and_then(|t| t.strip_prefix('#').map(|n| n.to_string()))
			.filter(|n| n.parse::<f32>().is_ok());

		let raw_name = if let Some(ref ep) = ep_badge {
			let ep_title = row
				.select_first("span.truncate")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			match ep_title {
				Some(t) => format!("Episode {ep}: {t}"),
				None => format!("Episode {ep}"),
			}
		} else {
			row.select_first(
				".chapter-number, .ch-name, .chapter-side-title, label, span > span:first-child",
			)
			.and_then(|el| el.text())
			.or_else(|| row.attr("data-title"))
			.map(|t| clean(&t))
			.unwrap_or_default()
		};

		let stamp = row
			.select_first(".chapter-date")
			.and_then(|el| el.text())
			.or_else(|| {
				if ep_badge.is_some() {
					row.select("span")
						.and_then(|mut spans| spans.next_back())
						.and_then(|el| el.text())
				} else {
					row.select("span > span")
						.and_then(|mut spans| spans.next_back())
						.and_then(|el| el.text())
				}
			})
			.map(|t| clean(&t));

		let chapter_number = ep_badge
			.as_deref()
			.and_then(|s| s.parse::<f32>().ok())
			.or_else(|| chapter_number_of(&raw_name));
		let title = clean_chapter_title(&raw_name, chapter_number);
		let date_uploaded = stamp.as_deref().and_then(parse_rinko_date);
		let url = if !base_key.starts_with("locked-") {
			Some(format!("{BASE_URL}/{base_key}"))
		} else {
			None
		};

		out.push(Chapter {
			key,
			title,
			chapter_number,
			date_uploaded,
			locked,
			url,
			..Default::default()
		});
	}
}

pub fn collect_chapters(doc: &Document, body: &str, url: &str, show_locked: bool) -> Vec<Chapter> {
	let mut chapters: Vec<Chapter> = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	parse_rows(doc, show_locked, &mut chapters, &mut seen);

	let initial_count = doc
		.select(CHAPTER_SELECTOR)
		.map(|rows| rows.count())
		.unwrap_or(0);

	if let Some(button) = doc.select_first(LOAD_MORE_SELECTOR)
		&& let Some(comic_id) = button
			.attr("data-comic-id")
			.map(|s| clean(&s))
			.filter(|s| !s.is_empty())
		&& let Some(nonce) = parse_nonce(body)
	{
		let mut offset = button
			.attr("data-offset")
			.and_then(|s| s.trim().parse::<usize>().ok())
			.unwrap_or(initial_count);
		for _ in 0..MAX_CHAPTER_PAGES {
			let mut form = QueryParameters::new();
			form.push("action", Some("load_more_chapters"));
			form.push("nonce", Some(&nonce));
			form.push("comic_id", Some(&comic_id));
			form.push("offset", Some(&offset.to_string()));

			let Ok(req) = Request::post(format!("{BASE_URL}/wp-admin/admin-ajax.php")) else {
				break;
			};
			let Ok(resp) = req
				.header("Referer", url)
				.header("X-Requested-With", "XMLHttpRequest")
				.header("Content-Type", "application/x-www-form-urlencoded")
				.body(form.to_string())
				.send()
			else {
				break;
			};
			let Ok(ajax) = resp.get_json_owned::<AjaxResponse>() else {
				break;
			};
			if ajax.success != Some(true) {
				break;
			}
			let Some(fragment) = ajax
				.data
				.and_then(|d| d.html)
				.filter(|h| !h.trim().is_empty())
			else {
				break;
			};
			let Ok(parsed) = Html::parse_with_url(&fragment, url) else {
				break;
			};
			let batch_rows = parsed
				.select(CHAPTER_SELECTOR)
				.map(|rows| rows.count())
				.unwrap_or(0);
			if batch_rows == 0 {
				break;
			}
			parse_rows(&parsed, show_locked, &mut chapters, &mut seen);
			offset += batch_rows.max(CHAPTERS_PER_PAGE);
			if batch_rows < CHAPTERS_PER_PAGE {
				break;
			}
		}
	}

	// Ensure newest-first order across comics and novels alike.
	chapters.sort_unstable_by(|a, b| match (b.chapter_number, a.chapter_number) {
		(Some(bn), Some(an)) => bn.total_cmp(&an),
		(Some(_), None) => core::cmp::Ordering::Less,
		(None, Some(_)) => core::cmp::Ordering::Greater,
		(None, None) => core::cmp::Ordering::Equal,
	});
	chapters
}

pub fn decode_cw_pages(encoded: &str) -> Result<Vec<String>> {
	let compact: String = encoded.chars().filter(|c| !c.is_whitespace()).collect();
	let bytes = BASE64_STANDARD
		.decode(compact.as_bytes())
		.map_err(|_| error!("Failed to decode chapter page list"))?;
	if bytes.len() <= 16 {
		bail!("Chapter page list is empty");
	}
	let mask = &bytes[..16];
	let decoded: Vec<u8> = bytes[16..]
		.iter()
		.enumerate()
		.map(|(i, b)| b ^ mask[i % 16])
		.collect();
	let pages: Vec<String> = serde_json::from_slice(&decoded)
		.map_err(|_| error!("Failed to parse chapter page list"))?;
	Ok(pages
		.into_iter()
		.filter(|u| u.starts_with("http://") || u.starts_with("https://"))
		.collect())
}
