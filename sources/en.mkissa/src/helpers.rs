use crate::models::*;
use aidoku::{
	Chapter, ContentRating, MangaStatus, Result, Viewer,
	alloc::{String, Vec},
	imports::std::{parse_date, parse_local_date},
	prelude::*,
};

pub fn decode_entities(input: &str) -> String {
	input
		.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&#39;", "'")
		.replace("&apos;", "'")
		.replace("&nbsp;", " ")
}

// The site shows the English name when a series has one
pub fn display_title(english: Option<&str>, name: &str) -> Result<String> {
	let title = english
		.map(str::trim)
		.filter(|english| !english.is_empty())
		.unwrap_or(name.trim());
	if title.is_empty() {
		bail!("Series has no title");
	}
	Ok(decode_entities(title))
}

// Tags arrive as slugs such as `format:long_strip`
pub fn tag_names(tags: Vec<String>) -> Vec<String> {
	let mut names: Vec<String> = tags
		.into_iter()
		.map(|tag| tag_name(&tag))
		.filter(|name| !name.is_empty())
		.collect();
	names.sort_unstable_by_key(|name| name.to_lowercase());
	names.dedup_by(|a, b| a.eq_ignore_ascii_case(b));
	names
}

pub fn tag_name(slug: &str) -> String {
	slug.rsplit(':')
		.next()
		.unwrap_or_default()
		.trim_end_matches("-magazine")
		.replace('_', " ")
		.split_whitespace()
		.map(|word| {
			let mut chars = word.chars();
			chars
				.next()
				.map(|first| first.to_uppercase().chain(chars).collect::<String>())
				.unwrap_or_default()
		})
		.collect::<Vec<_>>()
		.join(" ")
}

// Korean and Chinese comics are long strips; a series without a type falls back to its country
pub fn viewer(kind: Option<&str>, country: Option<&str>, tags: &[String]) -> Viewer {
	let kind = kind.map(str::to_lowercase);
	if tags
		.iter()
		.any(|tag| tag.eq_ignore_ascii_case("Long Strip") || tag.eq_ignore_ascii_case("Webtoons"))
	{
		return Viewer::Webtoon;
	}
	match (kind.as_deref(), country) {
		(Some("manhwa" | "manhua"), _) => Viewer::Webtoon,
		(Some("manga"), _) => Viewer::RightToLeft,
		(None, Some("KR" | "CN")) => Viewer::Webtoon,
		(None, Some("JP")) => Viewer::RightToLeft,
		_ => Viewer::Unknown,
	}
}

// Thumbnails are paths on the image host, or full urls for covers taken from elsewhere
pub fn cover_url(thumbnail: Option<&str>) -> Option<String> {
	let thumbnail = thumbnail.map(str::trim).filter(|path| !path.is_empty())?;
	if is_absolute(thumbnail) {
		Some(thumbnail.into())
	} else {
		Some(format!(
			"{CDN_URL}/aln.youtube-anime.com/{}?w=250",
			thumbnail.trim_start_matches('/')
		))
	}
}

pub fn content_rating(genres: &[String], is_adult: Option<bool>) -> ContentRating {
	let has = |names: &[&str]| {
		genres
			.iter()
			.any(|genre| names.iter().any(|name| genre.eq_ignore_ascii_case(name)))
	};
	if is_adult == Some(true) || has(&["Adult", "Hentai", "Smut", "Yaoi"]) {
		ContentRating::NSFW
	} else if has(&["Ecchi", "Mature"]) {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

pub fn parse_status(status: Option<&str>) -> MangaStatus {
	let status = status.unwrap_or_default().to_lowercase();
	if status.contains("releasing") || status.contains("ongoing") {
		MangaStatus::Ongoing
	} else if status.contains("finished") || status.contains("completed") {
		MangaStatus::Completed
	} else if status.contains("hiatus") {
		MangaStatus::Hiatus
	} else if status.contains("cancel") {
		MangaStatus::Cancelled
	} else {
		MangaStatus::Unknown
	}
}

// Descriptions are HTML with `<br>` line breaks
pub fn strip_html(html: &str) -> String {
	let mut out = String::with_capacity(html.len());
	let mut in_tag = false;
	let normalized = html
		.replace("<br>", "\n")
		.replace("<br/>", "\n")
		.replace("<br />", "\n")
		.replace("</p>", "\n\n");
	for ch in normalized.chars() {
		match ch {
			'<' => in_tag = true,
			'>' => in_tag = false,
			_ if !in_tag => out.push(ch),
			_ => {}
		}
	}
	decode_entities(out.trim())
}

pub fn apply_image_quality(url: &str, quality: &str) -> String {
	if quality == "original" {
		return url.into();
	}
	match url
		.strip_prefix("https://")
		.or_else(|| url.strip_prefix("http://"))
	{
		Some(rest) => {
			let path = rest.split('#').next().unwrap_or(rest);
			format!("{CDN_URL}/{path}?w={quality}")
		}
		None => url.into(),
	}
}

// Date parts count months from zero
pub fn date_from_parts(parts: &DateParts) -> Option<i64> {
	let formatted = format!(
		"{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
		parts.year?,
		parts.month.unwrap_or(0) + 1,
		parts.date.unwrap_or(1),
		parts.hour.unwrap_or(0),
		parts.minute.unwrap_or(0),
		parts.second.unwrap_or(0)
	);
	parse_date(formatted, "yyyy-MM-dd HH:mm:ss")
}

// The site names its season rows by calendar quarter: Winter is January to March
pub fn season_and_year(timestamp: i64) -> (&'static str, i64) {
	let (year, month, _) = civil_date(timestamp);
	let season = ["Winter", "Spring", "Summer", "Fall"][((month - 1) / 3) as usize];
	(season, year)
}

// The site's update days run from local midnight to the second before the next
pub fn day_range(days_ago: i64, now: i64) -> Option<(i64, i64)> {
	let (year, month, day) = civil_date(now);
	let noon = format!("{year:04}-{month:02}-{day:02} 12:00:00");
	let offset =
		parse_date(&noon, "yyyy-MM-dd HH:mm:ss")? - parse_local_date(&noon, "yyyy-MM-dd HH:mm:ss")?;
	let start = (now + offset).div_euclid(86400) * 86400 - offset - days_ago * 86400;
	Some((start, start + 86399))
}

// Days to civil date, after Howard Hinnant's algorithm
fn civil_date(timestamp: i64) -> (i64, i64, i64) {
	let days = timestamp.div_euclid(86400) + 719_468;
	let era = days.div_euclid(146_097);
	let day_of_era = days - era * 146_097;
	let year_of_era =
		(day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365;
	let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
	let month_index = (5 * day_of_year + 2) / 153;
	let month = if month_index < 10 {
		month_index + 3
	} else {
		month_index - 9
	};
	let day = day_of_year - (153 * month_index + 2) / 5 + 1;
	(year_of_era + era * 400 + i64::from(month <= 2), month, day)
}

pub fn has_next_page(page: i32, total: Option<i32>) -> bool {
	total.is_some_and(|total| page * PAGE_SIZE < total)
}

// Episode numbers arrive as numbers or strings
fn is_episode(value: &serde_json::Value, number: &str) -> bool {
	match value {
		serde_json::Value::String(value) => value == number,
		serde_json::Value::Number(value) => value.as_f64() == number.parse().ok(),
		_ => false,
	}
}

// A chapter note reads "[S2] ep. 12 - Title"; keep only the title
fn chapter_title(notes: &str) -> Option<String> {
	let trimmed = notes.trim();
	let without_season = match trimmed
		.strip_prefix('[')
		.and_then(|rest| rest.split_once(']'))
	{
		Some((_, rest)) => rest.trim_start(),
		None => trimmed,
	};
	let title = strip_ep_prefix(without_season).trim();
	title
		.chars()
		.any(|c| c.is_ascii_alphabetic())
		.then(|| decode_entities(title))
}

fn strip_ep_prefix(input: &str) -> &str {
	if !input
		.get(..2)
		.is_some_and(|prefix| prefix.eq_ignore_ascii_case("ep"))
	{
		return input;
	}
	let rest = input[2..].trim_start_matches('.').trim_start();
	let digits = rest
		.find(|c: char| !(c.is_ascii_digit() || c == '.'))
		.unwrap_or(rest.len());
	if digits == 0 {
		return input;
	}
	rest[digits..]
		.trim_start()
		.trim_start_matches('-')
		.trim_start()
}

// The chapter list arrives newest first
pub fn parse_chapters(data: ChaptersData, manga_key: &str) -> Vec<Chapter> {
	let infos = data.episode_infos.unwrap_or_default();
	data.manga
		.and_then(|manga| manga.available_chapters_detail)
		.and_then(|detail| detail.sub)
		.unwrap_or_default()
		.into_iter()
		.map(|number| {
			let info = infos
				.iter()
				.find(|info| is_episode(&info.episode_id_num, &number));
			Chapter {
				title: info
					.and_then(|info| info.notes.as_deref())
					.and_then(chapter_title),
				chapter_number: number.parse().ok(),
				date_uploaded: info
					.and_then(|info| info.upload_dates.as_ref())
					.and_then(|dates| dates.sub.as_deref())
					.and_then(|date| date.get(..19))
					.and_then(|date| parse_date(date, "yyyy-MM-dd'T'HH:mm:ss")),
				url: Some(format!("{BASE_URL}/manga/{manga_key}/chapter-{number}-sub")),
				key: number,
				..Default::default()
			}
		})
		.collect()
}

fn is_absolute(url: &str) -> bool {
	url.starts_with("http://") || url.starts_with("https://")
}

pub fn parse_page_urls(pages: &ChapterPages, quality: &str) -> Vec<String> {
	let Some(first) = pages.edges.first() else {
		return Vec::new();
	};
	let edge = pages
		.edges
		.iter()
		.find(|edge| {
			edge.image_host().is_some()
				|| edge
					.picture_urls
					.iter()
					.flatten()
					.any(|picture| picture.url().is_some_and(is_absolute))
		})
		.unwrap_or(first);
	let image_host = match edge.image_host() {
		Some(server) if is_absolute(server) => format!("{}/", server.trim_end_matches('/')),
		Some(server) => format!("https://{}/", server.trim_end_matches('/')),
		None => "https://ytimgf.youtube-anime.com/".into(),
	};
	edge.picture_urls
		.iter()
		.flatten()
		.filter_map(PictureUrl::url)
		.filter(|url| !url.is_empty())
		.map(|url| {
			let url = if is_absolute(url) {
				url.into()
			} else {
				format!("{image_host}{}", url.trim_start_matches('/'))
			};
			apply_image_quality(&url, quality)
		})
		.collect()
}
