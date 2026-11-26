use crate::{
	api::BASE_URL,
	helpers::{app_language, cover_url, names},
};
use aidoku::{
	AidokuError, Chapter, ContentRating, Manga, MangaStatus, Result, Viewer,
	alloc::{String, Vec, collections::BTreeSet, format, vec},
	helpers::string::PlainText,
	imports::std::parse_date,
	prelude::*,
};
use serde::{Deserialize, Deserializer};

// Novels share the comic catalogue's keys, so theirs are prefixed.
pub const NOVEL_PREFIX: &str = "novel/";

#[derive(Deserialize)]
pub struct Listing<T> {
	pub data: Vec<T>,
	pub total: Option<i64>,
}

// One shape serves listings, the home rows and the comic page.
#[derive(Deserialize)]
pub struct Title {
	title: Option<String>,
	#[serde(default)]
	url: String,
	cover: Option<String>,
	#[serde(default)]
	covers: Vec<String>,
	description: Option<String>,
	#[serde(default, deserialize_with = "list")]
	pub genres: Vec<String>,
	#[serde(default, deserialize_with = "list")]
	pub tags: Vec<String>,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	status: Option<String>,
	author: Option<String>,
	#[serde(default)]
	authors: Vec<String>,
	#[serde(default)]
	artists: Vec<String>,
	nsfw: Option<bool>,
	#[serde(default, rename = "availableLangs", alias = "available_langs")]
	pub languages: Vec<String>,
	#[serde(rename = "chapterCount")]
	pub chapter_count: Option<f32>,
}

#[derive(Deserialize)]
pub struct Update {
	title: Option<String>,
	pub url: String,
	cover: Option<String>,
	chapter_id: String,
	number: String,
	pub lang: String,
	released: Option<String>,
	source_name: Option<String>,
}

#[derive(Deserialize)]
pub struct ComicPage {
	pub manga: Title,
	pub chapters: Vec<ChapterItem>,
}

#[derive(Deserialize)]
pub struct Genres {
	pub genres: Vec<String>,
}

#[derive(Deserialize)]
pub struct ChapterItem {
	pub id: String,
	lang: String,
	pub number: String,
	title: Option<String>,
	pub rank: Option<String>,
	published_date: Option<String>,
	source_name: Option<String>,
	source: Option<String>,
	scan_group: Option<String>,
}

#[derive(Deserialize)]
pub struct Response<T> {
	pub data: T,
}

#[derive(Deserialize)]
pub struct Novel {
	title: String,
	slug: String,
	author: Option<String>,
	description: Option<String>,
	#[serde(default)]
	covers: Vec<String>,
	status: Option<String>,
	#[serde(default)]
	pub chapters: Vec<NovelChapter>,
}

#[derive(Deserialize)]
pub struct NovelChapter {
	number: f32,
	title: Option<String>,
	#[serde(rename = "createdAt")]
	created_at: Option<String>,
}

#[derive(Deserialize)]
pub struct NovelText {
	pub content: String,
}

#[derive(Deserialize)]
pub struct Pages {
	#[serde(rename = "baseUrl")]
	base_url: Option<String>,
	chapter: PageUrls,
}

#[derive(Deserialize)]
struct PageUrls {
	data: Vec<String>,
}

impl Pages {
	pub fn into_urls(self) -> Vec<String> {
		let base = self.base_url.unwrap_or_default();
		self.chapter
			.data
			.into_iter()
			.map(|path| {
				if path.is_empty() || path.starts_with("https://") || path.starts_with("http://") {
					path
				} else if path.starts_with('/') {
					format!("{BASE_URL}{path}")
				} else if base.starts_with("https://") || base.starts_with("http://") {
					format!("{}/{path}", base.trim_end_matches('/'))
				} else {
					format!("{BASE_URL}{}/{path}", base.trim_end_matches('/'))
				}
			})
			.collect()
	}
}

// Some records hold their lists as Python reprs ("['Action', \"Boys' Love\"]").
fn list<'de, D: Deserializer<'de>>(deserializer: D) -> core::result::Result<Vec<String>, D::Error> {
	#[derive(Deserialize)]
	#[serde(untagged)]
	enum List {
		Many(Vec<String>),
		One(String),
	}
	Ok(match Option::<List>::deserialize(deserializer)? {
		Some(List::Many(values)) => values,
		Some(List::One(text)) => {
			let mut values = Vec::new();
			let mut chars = text.chars();
			while let Some(c) = chars.next() {
				if c == '\'' || c == '"' {
					values.push(chars.by_ref().take_while(|next| *next != c).collect());
				}
			}
			values
		}
		None => Vec::new(),
	})
}

fn status(status: Option<&str>) -> MangaStatus {
	match status.map(str::to_lowercase).as_deref() {
		Some("ongoing") => MangaStatus::Ongoing,
		Some("completed") => MangaStatus::Completed,
		Some("hiatus") => MangaStatus::Hiatus,
		Some("cancelled" | "discontinued") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

// "0001-01-01" marks an unknown date; avoid quoted literals, which the runner does not parse.
fn date(value: Option<&str>) -> Option<i64> {
	let value = value.filter(|value| !value.starts_with("0001"))?;
	let day = value.get(..10)?;
	let time = value.get(11..19).unwrap_or("00:00:00");
	parse_date(format!("{day} {time}"), "yyyy-MM-dd HH:mm:ss")
}

impl Title {
	pub fn content_rating(&self) -> ContentRating {
		let has = |names: &[&str]| {
			self.genres
				.iter()
				.chain(&self.tags)
				.any(|tag| names.contains(&tag.as_str()))
		};
		if self.nsfw == Some(true) || has(&["Adult", "Hentai", "Pornographic", "Smut"]) {
			ContentRating::NSFW
		} else if has(&["Ecchi", "Mature"]) {
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		}
	}

	fn into_manga(self, detailed: bool) -> Result<Manga> {
		let content_rating = self.content_rating();
		let title = self
			.title
			.filter(|name| !name.trim().is_empty())
			.ok_or_else(|| error!("Series has no title"))?;
		if self.url.is_empty() {
			bail!("Series has no key");
		}
		let mut manga = Manga {
			cover: self
				.covers
				.into_iter()
				.chain(self.cover)
				.find_map(|cover| cover_url(&cover)),
			content_rating,
			..Default::default()
		};
		if detailed {
			manga.viewer = match self.kind.as_deref().map(str::to_lowercase).as_deref() {
				Some("manhwa" | "manhua") => Viewer::Webtoon,
				_ if self.tags.iter().any(|tag| tag == "Long Strip") => Viewer::Webtoon,
				Some("manga") => Viewer::RightToLeft,
				_ => Viewer::Unknown,
			};
			let mut tags = self.genres;
			tags.extend(self.tags);
			tags.sort_unstable();
			tags.dedup();
			manga.url = Some(format!("{BASE_URL}/comic/{}", self.url));
			manga.description = self
				.description
				.filter(|text| !text.trim().is_empty())
				.map(|text| text.escape_markdown());
			manga.authors = names(self.authors).or_else(|| names(self.author));
			manga.artists = names(self.artists);
			manga.status = status(self.status.as_deref());
			manga.tags = (!tags.is_empty()).then_some(tags);
		}
		manga.key = self.url;
		manga.title = title;
		Ok(manga)
	}

	pub fn into_details(self) -> Result<Manga> {
		self.into_manga(true)
	}
}

impl TryFrom<Title> for Manga {
	type Error = AidokuError;

	fn try_from(title: Title) -> Result<Self> {
		title.into_manga(false)
	}
}

impl Update {
	pub fn uploader(&self) -> &str {
		self.source_name.as_deref().unwrap_or("swarmws")
	}

	pub fn into_manga(self) -> Option<Manga> {
		Some(Manga {
			key: self.url,
			title: self.title.filter(|title| !title.trim().is_empty())?,
			cover: self.cover.as_deref().and_then(cover_url),
			..Default::default()
		})
	}

	pub fn into_chapter(mut self) -> Option<(Manga, Chapter)> {
		let chapter = Chapter {
			key: core::mem::take(&mut self.chapter_id),
			chapter_number: self.number.parse().ok(),
			date_uploaded: date(self.released.as_deref()),
			scanlators: Some(vec![self.uploader().into()]),
			language: Some(app_language(&self.lang).into()),
			..Default::default()
		};
		Some((self.into_manga()?, chapter))
	}
}

// The site's scrapers occasionally mislabel non-English chapters as "en".
fn effective_language<'a>(lang: &'a str, title: Option<&str>) -> &'a str {
	let mapped = app_language(lang);
	if mapped != "en" {
		return mapped;
	}
	let Some(line) = title.and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty())) else {
		return mapped;
	};
	let lower = line.to_lowercase();
	for (prefix, code) in [
		("chương", "vi"),
		("chuong", "vi"),
		("ตอนที่", "th"),
		("บทที่", "th"),
		("capítulo", "es"),
		("capitulo", "es"),
		("chapitre", "fr"),
		("kapitel", "de"),
		("глава", "ru"),
		("الفصل", "ar"),
		("bölüm", "tr"),
		("bolum", "tr"),
		("rozdział", "pl"),
		("rozdzial", "pl"),
		("capitolo", "it"),
		("bab ", "id"),
	] {
		if lower.starts_with(prefix) {
			return code;
		}
	}
	let has_ascii_alpha = line.chars().any(|c| c.is_ascii_alphabetic());
	if !has_ascii_alpha && line.chars().any(|c| c.is_alphabetic()) {
		for ch in line.chars() {
			match ch as u32 {
				0x0E00..=0x0E7F => return "th",
				0x0600..=0x06FF => return "ar",
				0x0400..=0x04FF => return "ru",
				0xAC00..=0xD7AF | 0x1100..=0x11FF => return "ko",
				0x3040..=0x30FF => return "ja",
				0x4E00..=0x9FFF => return "zh",
				_ => {}
			}
		}
		return "und";
	}
	mapped
}

fn strip_prefix_ci<'a>(text: &'a str, prefix: &str) -> Option<&'a str> {
	let (head, tail) = text.split_at_checked(prefix.len())?;
	head.eq_ignore_ascii_case(prefix).then_some(tail)
}

fn split_leading_number(text: &str) -> Option<(&str, &str)> {
	let mut end = 0;
	let mut seen_dot = false;
	for (i, ch) in text.char_indices() {
		if ch.is_ascii_digit() {
			end = i + ch.len_utf8();
		} else if ch == '.' && !seen_dot && end > 0 {
			seen_dot = true;
			end = i + 1;
		} else {
			break;
		}
	}
	let num = text[..end].trim_end_matches('.');
	(!num.is_empty())
		.then(|| text.strip_prefix(num).map(|tail| (num, tail)))
		.flatten()
}

// Swarm's scraper sometimes drops trailing zeroes from integer chapter numbers (e.g. "Chapter 50" -> "5").
fn is_trailing_zero_truncation(raw: &str, raw_num: f32, title_str: &str, title_num: f32) -> bool {
	if raw.contains('.') || title_str.contains('.') || title_num <= raw_num {
		return false;
	}
	let raw_sig = raw.trim_start_matches('0').trim_end_matches('0');
	let title_sig = title_str.trim_start_matches('0').trim_end_matches('0');
	!raw_sig.is_empty() && raw_sig == title_sig
}

fn parse_chapter_label(line: &str) -> (Option<f32>, Option<(&str, f32, &str)>) {
	let mut rest = line.trim();
	let mut volume = None;
	for prefix in ["volume", "vol.", "vol"] {
		if let Some(after) = strip_prefix_ci(rest, prefix)
			&& (after.is_empty()
				|| after.starts_with(|c: char| c.is_whitespace() || c.is_ascii_digit()))
			&& let Some((vol_str, tail)) = split_leading_number(after.trim_start())
			&& let Ok(v) = vol_str.parse::<f32>()
		{
			volume = Some(v);
			rest = tail
				.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '-' | ':' | ','));
			break;
		}
	}
	for prefix in ["chapter", "ch.", "ch", "episode", "ep.", "ep", "#"] {
		if let Some(after) = strip_prefix_ci(rest, prefix)
			&& (prefix == "#"
				|| after.starts_with(|c: char| c.is_whitespace() || c.is_ascii_digit() || c == '.'))
			&& let Some((num_str, tail)) =
				split_leading_number(after.trim_start_matches([' ', '.']))
			&& let Ok(num) = num_str.parse::<f32>()
		{
			return (volume, Some((num_str, num, tail)));
		}
	}
	if let Some((num_str, tail)) = split_leading_number(rest)
		&& tail.trim().is_empty()
		&& let Ok(num) = num_str.parse::<f32>()
	{
		return (volume, Some((num_str, num, tail)));
	}
	(volume, None)
}

fn clean_title_and_volume(raw: Option<&str>, number: Option<f32>) -> (Option<f32>, Option<String>) {
	let Some(mut line) = raw.and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty())) else {
		return (None, None);
	};
	let mut volume = None;
	loop {
		let (vol, parsed) = parse_chapter_label(line);
		if volume.is_none() {
			volume = vol;
		}
		if let Some((_, label_num, tail)) = parsed
			&& number.is_some_and(|n| (n - label_num).abs() < f32::EPSILON)
		{
			let cleaned = tail
				.trim_start_matches(|c: char| {
					c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—' | '|')
				})
				.trim();
			if cleaned.is_empty() {
				return (volume, None);
			}
			line = cleaned;
			continue;
		}
		return (volume, Some(String::from(line)));
	}
}

fn canonical_number_key(number: Option<f32>, raw: &str) -> core::result::Result<u32, &str> {
	match number {
		Some(n) if n.is_finite() => Ok(if n == 0.0 { 0.0_f32 } else { n }.to_bits()),
		_ => Err(raw),
	}
}

impl ChapterItem {
	fn uploader(&self) -> &str {
		[&self.source_name, &self.source, &self.scan_group]
			.into_iter()
			.flatten()
			.map(|name| name.as_str())
			.find(|name| !name.trim().is_empty())
			.unwrap_or("swarmws")
	}

	fn group(&self) -> String {
		self.uploader()
			.trim_end_matches(|c: char| c.is_ascii_digit())
			.to_lowercase()
	}

	fn site_order(&self) -> (&str, u32) {
		let uploader = self.uploader();
		let copy = uploader
			.strip_prefix(uploader.trim_end_matches(|c: char| c.is_ascii_digit()))
			.and_then(|digits| digits.parse().ok())
			.unwrap_or(1);
		(self.rank.as_deref().unwrap_or("~"), copy)
	}

	fn language(&self) -> &str {
		effective_language(&self.lang, self.title.as_deref())
	}

	fn effective_number(&self) -> Option<f32> {
		let raw = self.number.trim();
		let raw_num = raw.parse::<f32>().ok();
		if let Some(line) = self
			.title
			.as_deref()
			.and_then(|t| t.lines().map(str::trim).find(|l| !l.is_empty()))
			&& let (_, Some((title_str, title_num, tail))) = parse_chapter_label(line)
			&& tail.trim().is_empty()
			&& (raw_num.is_none()
				|| raw_num
					.is_some_and(|n| is_trailing_zero_truncation(raw, n, title_str, title_num)))
		{
			return Some(title_num);
		}
		raw_num
	}

	fn into_chapter(self, slug: &str) -> Chapter {
		let number = self.effective_number();
		let language = String::from(self.language());
		let (volume_number, title) = clean_title_and_volume(self.title.as_deref(), number);
		let scanlators = Some(vec![self.uploader().into()]);
		let query = self
			.rank
			.as_deref()
			.filter(|rank| *rank != "a")
			.map(|rank| format!("?{rank}"))
			.unwrap_or_default();
		Chapter {
			url: Some(format!(
				"{BASE_URL}/comic/{slug}/chapter/{}{query}",
				self.number
			)),
			title,
			chapter_number: number,
			volume_number,
			date_uploaded: date(self.published_date.as_deref()),
			scanlators,
			language: Some(language),
			key: self.id,
			..Default::default()
		}
	}
}

pub fn chapter_list(
	items: Vec<ChapterItem>,
	slug: &str,
	languages: &[String],
	blocked: &[String],
	mode: &str,
	preferred: &[String],
) -> Vec<Chapter> {
	let listed = |names: &[String], item: &ChapterItem| {
		names
			.iter()
			.position(|name| *name == item.uploader().to_lowercase() || *name == item.group())
	};
	let mut items: Vec<ChapterItem> = items
		.into_iter()
		.filter(|item| {
			let lang = item.language();
			languages.iter().any(|language| language == lang)
		})
		.filter(|item| listed(blocked, item).is_none())
		.collect();
	let preference = |item: &ChapterItem| {
		if mode == "chapter" {
			listed(preferred, item).unwrap_or(preferred.len())
		} else {
			0
		}
	};
	items.sort_unstable_by(|a, b| {
		let num_cmp = match (b.effective_number(), a.effective_number()) {
			(Some(bn), Some(an)) => bn.total_cmp(&an),
			(Some(_), None) => core::cmp::Ordering::Less,
			(None, Some(_)) => core::cmp::Ordering::Greater,
			(None, None) => b.number.cmp(&a.number),
		};
		num_cmp
			.then_with(|| preference(a).cmp(&preference(b)))
			.then_with(|| a.site_order().cmp(&b.site_order()))
			.then_with(|| a.id.cmp(&b.id))
	});
	if mode == "off" {
		return items
			.into_iter()
			.map(|item| item.into_chapter(slug))
			.collect();
	}
	let mut seen = BTreeSet::new();
	items
		.into_iter()
		.filter(|item| {
			let group = if mode == "uploader" {
				item.group()
			} else {
				String::new()
			};
			let key = (
				String::from(item.language()),
				canonical_number_key(item.effective_number(), &item.number).map_err(String::from),
				group,
			);
			seen.insert(key)
		})
		.map(|item| item.into_chapter(slug))
		.collect()
}

impl Novel {
	pub fn into_manga(self) -> Option<Manga> {
		Some(Manga {
			key: format!("{NOVEL_PREFIX}{}", self.slug),
			cover: self.covers.first().and_then(|cover| cover_url(cover)),
			title: Some(self.title).filter(|title| !title.trim().is_empty())?,
			..Default::default()
		})
	}

	pub fn into_details(mut self) -> Result<Manga> {
		let url = format!("{BASE_URL}/novel/{}", self.slug);
		let mut raw_chapters = core::mem::take(&mut self.chapters);
		raw_chapters.sort_unstable_by(|a, b| b.number.total_cmp(&a.number));
		let chapters = raw_chapters
			.into_iter()
			.map(|chapter| {
				let number = format!("{}", chapter.number);
				let (volume_number, title) =
					clean_title_and_volume(chapter.title.as_deref(), Some(chapter.number));
				Chapter {
					url: Some(format!("{url}/chapter/{number}")),
					title,
					chapter_number: Some(chapter.number),
					volume_number,
					date_uploaded: date(chapter.created_at.as_deref()),
					language: Some("en".into()),
					key: number,
					..Default::default()
				}
			})
			.collect();
		Ok(Manga {
			url: Some(url),
			description: self
				.description
				.take()
				.filter(|text| !text.trim().is_empty())
				.map(|text| text.escape_markdown()),
			authors: names(self.author.take()),
			status: status(self.status.as_deref()),
			viewer: Viewer::Unknown,
			chapters: Some(chapters),
			..self
				.into_manga()
				.ok_or_else(|| error!("Novel has no title"))?
		})
	}
}
