use crate::{
	rsc::{Classified, Flight, slug_like},
	settings, tokens,
};
use aidoku::{
	Chapter, ContentRating, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	ListingKind, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Result, Viewer,
	alloc::{
		String, Vec,
		collections::{BTreeMap, BTreeSet},
		format,
		string::ToString,
	},
	helpers::string::PlainText,
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
	prelude::*,
};
use serde_json::{Map, Value};

pub const BASE_URL: &str = "https://templetoons.com";
const PAGE_SIZE: usize = 20;

const CATALOG_KNOWN: [&str; 7] = [
	"thumbnail",
	"badge",
	"status",
	"created_at",
	"alternative_names",
	"update_chapter",
	"total_views",
];
const HOME_KNOWN: [&str; 11] = [
	"id",
	"description",
	"thumbnail",
	"badge",
	"protagonist",
	"banner",
	"total_views",
	"day_views",
	"week_views",
	"month_views",
	"slug",
];

pub struct CatalogItem {
	pub slug: String,
	pub title: String,
	pub cover: Option<String>,
	pub badge: Option<String>,
	pub status: Option<String>,
	pub alternative_names: Option<String>,
	pub created_at: Option<i64>,
	pub updated_at: Option<i64>,
	pub views: u64,
}

pub struct ReaderData {
	pub tokens: Vec<String>,
	pub script: Option<(u64, String)>,
}

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn image_url(path: Option<&str>) -> Option<String> {
	let path = path?.trim();
	if path.is_empty() {
		return None;
	}
	Some(
		if path.starts_with("http://") || path.starts_with("https://") {
			path.to_string()
		} else if path.starts_with('/') {
			format!("{BASE_URL}{path}")
		} else {
			format!("{BASE_URL}/{path}")
		},
	)
}

fn parse_status(status: Option<&str>) -> MangaStatus {
	match status.unwrap_or("").trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("completed") => MangaStatus::Completed,
		s if s.contains("hiatus") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("drop") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

fn rating_of(label: &str) -> ContentRating {
	let lower = label.to_ascii_lowercase();
	if lower.contains("+18")
		|| lower.contains("18+")
		|| lower.contains("adult")
		|| lower.contains("smut")
		|| lower.contains("hentai")
	{
		ContentRating::NSFW
	} else if lower.contains("mature") || lower.contains("ecchi") {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

fn strip_html(html: &str) -> String {
	let normalized = html.replace("<br>", "\n").replace("<br/>", "\n");
	let mut out = String::with_capacity(normalized.len());
	let mut in_tag = false;
	for ch in normalized.chars() {
		match ch {
			'<' => in_tag = true,
			'>' => in_tag = false,
			_ if !in_tag => out.push(ch),
			_ => {}
		}
	}
	let text = match out.find('#') {
		Some(idx) => out[..idx].trim_end_matches([' ', ':', '\n']).to_string(),
		None => out,
	};
	text.replace("&nbsp;", " ")
		.replace("&amp;", "&")
		.replace("&#39;", "'")
		.replace("&quot;", "\"")
		.trim()
		.to_string()
}

fn parse_iso_utc(value: Option<&str>) -> Option<i64> {
	let raw = value?.trim();
	parse_date_with_options(raw, "yyyy-MM-dd'T'HH:mm:ss.SSSXXX", "en_US_POSIX", "UTC")
		.or_else(|| parse_date_with_options(raw, "yyyy-MM-dd'T'HH:mm:ssXXX", "en_US_POSIX", "UTC"))
		.or_else(|| {
			let naive = raw
				.split('.')
				.next()?
				.trim_end_matches("+00:00")
				.trim_end_matches('Z');
			parse_date_with_options(naive, "yyyy-MM-ddTHH:mm:ss", "en_US_POSIX", "UTC")
		})
		.or_else(|| parse_date_with_options(raw, "yyyy-MM-dd", "en_US_POSIX", "UTC"))
}

fn first_number(text: &str) -> Option<f32> {
	let mut digits = String::new();
	let mut seen_dot = false;
	for ch in text.chars() {
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

fn strip_chapter_label(raw: &str, number: Option<f32>) -> Option<String> {
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

pub fn parse_comic_href(href: &str) -> Option<(String, Option<String>)> {
	let after = href.split("/comic/").nth(1)?;
	let mut parts = after.split(['/', '?', '#']);
	let slug = parts.next().filter(|s| !s.is_empty())?;
	let chapter = parts.next().filter(|s| !s.is_empty());
	Some((slug.into(), chapter.map(String::from)))
}

pub fn is_challenge_page(doc: &Document) -> bool {
	if doc
		.select_first("title")
		.and_then(|el| el.text())
		.is_some_and(|t| {
			let lower = t.to_ascii_lowercase();
			lower.contains("just a moment")
				|| lower.contains("security check")
				|| lower.contains("verify")
				|| lower.contains("challenge")
		}) {
		return true;
	}
	doc.select_first(
		"script[src*='/app/challenge/'], script[src*='/challenge/'], \
		 script[src*='challenges.cloudflare.com'], #turnstile-widget, .cf-turnstile",
	)
	.is_some()
}

pub fn build_request(url: &str) -> Result<Request> {
	let mut request = Request::get(url)?.header("Referer", &format!("{BASE_URL}/"));
	if let Some(cookie) = settings::verify_cookie_header() {
		request = request.header("Cookie", &cookie);
	}
	Ok(request)
}

pub fn fetch_doc(url: &str) -> Result<Document> {
	let response = build_request(url)?.send()?;
	let status = response.status_code();
	if status == 403 || status == 503 {
		bail!(
			"Temple Scan requires verification; open Source Settings and tap Verify Temple Scan Captcha"
		);
	}
	let doc = response.get_html()?;
	if is_challenge_page(&doc) {
		bail!(
			"Temple Scan requires verification; open Source Settings and tap Verify Temple Scan Captcha"
		);
	}
	Ok(doc)
}

fn parse_views(val: Option<&Value>) -> u64 {
	match val {
		Some(Value::Number(n)) => n
			.as_u64()
			.or_else(|| n.as_f64().filter(|f| *f >= 0.0).map(|f| f as u64))
			.unwrap_or(0),
		Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
		_ => 0,
	}
}

fn parse_catalog_records(records: &[&Map<String, Value>]) -> Vec<CatalogItem> {
	let Some(fields) = Classified::of(records, &CATALOG_KNOWN) else {
		return Vec::new();
	};
	let mut seen = BTreeSet::new();
	records
		.iter()
		.filter_map(|record| {
			let slug = record.get(&fields.slug)?.as_str()?.trim();
			let title = record.get(&fields.text)?.as_str()?.trim();
			if slug.is_empty() || title.is_empty() || !seen.insert(slug) {
				return None;
			}
			Some(CatalogItem {
				slug: slug.into(),
				title: title.into(),
				cover: image_url(record.get("thumbnail").and_then(Value::as_str)),
				badge: record
					.get("badge")
					.and_then(Value::as_str)
					.map(|s| s.trim().into()),
				status: record
					.get("status")
					.and_then(Value::as_str)
					.map(|s| s.trim().into()),
				alternative_names: record
					.get("alternative_names")
					.and_then(Value::as_str)
					.map(String::from),
				created_at: parse_iso_utc(record.get("created_at").and_then(Value::as_str)),
				updated_at: parse_iso_utc(record.get("update_chapter").and_then(Value::as_str)),
				views: parse_views(record.get("total_views")),
			})
		})
		.collect()
}

pub fn parse_catalog(doc: &Document) -> Vec<CatalogItem> {
	let flight = Flight::from_document(doc);
	if let Some(props) = flight.find(&["list", "initialPage"])
		&& let Some(list) = props.get("list").and_then(Value::as_array)
	{
		let records: Vec<&Map<String, Value>> = list.iter().filter_map(Value::as_object).collect();
		let parsed = parse_catalog_records(&records);
		if !parsed.is_empty() {
			return parsed;
		}
	}
	let mut best = Vec::new();
	for array in flight.all_arrays() {
		let records: Vec<&Map<String, Value>> = array.iter().filter_map(Value::as_object).collect();
		if records.len() <= best.len()
			|| !records.first().is_some_and(|r| r.contains_key("thumbnail"))
		{
			continue;
		}
		let parsed = parse_catalog_records(&records);
		if parsed.len() > best.len() {
			best = parsed;
		}
	}
	best
}

pub fn sort_catalog(items: &mut [CatalogItem], sort: i32, ascending: bool) {
	items.sort_unstable_by(|a, b| {
		let ord = match sort {
			1 => a
				.created_at
				.unwrap_or(i64::MIN)
				.cmp(&b.created_at.unwrap_or(i64::MIN)),
			2 => a
				.title
				.to_ascii_lowercase()
				.cmp(&b.title.to_ascii_lowercase()),
			3 => a.views.cmp(&b.views),
			_ => a
				.updated_at
				.unwrap_or(i64::MIN)
				.cmp(&b.updated_at.unwrap_or(i64::MIN)),
		}
		.then_with(|| a.slug.cmp(&b.slug));
		if ascending { ord } else { ord.reverse() }
	});
}

pub fn page_catalog(items: Vec<CatalogItem>, page: i32) -> MangaPageResult {
	let start = (page.max(1) as usize - 1) * PAGE_SIZE;
	let has_next_page = items.len() > start + PAGE_SIZE;
	MangaPageResult {
		entries: items
			.into_iter()
			.skip(start)
			.take(PAGE_SIZE)
			.map(|item| Manga {
				url: Some(format!("{BASE_URL}/comic/{}", item.slug)),
				key: item.slug,
				title: item.title,
				cover: item.cover,
				status: parse_status(item.status.as_deref()),
				content_rating: item
					.badge
					.as_deref()
					.map(rating_of)
					.unwrap_or(ContentRating::Safe),
				..Default::default()
			})
			.collect(),
		has_next_page,
	}
}

fn extract_home_entry(flight: &Flight, record: &Map<String, Value>) -> Option<(String, String)> {
	let strings: Vec<&str> = record
		.iter()
		.filter(|(k, _)| !HOME_KNOWN.contains(&k.as_str()))
		.filter_map(|(_, v)| {
			v.as_str()
				.and_then(|s| flight.resolve_str(s))
				.map(str::trim)
				.filter(|s| !s.is_empty())
		})
		.collect();
	let slug = record
		.get("series_slug")
		.and_then(Value::as_str)
		.and_then(|s| flight.resolve_str(s))
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.or_else(|| strings.iter().copied().find(|s| slug_like(s)))?;
	let title = record
		.get("title")
		.and_then(Value::as_str)
		.and_then(|s| flight.resolve_str(s))
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.or_else(|| {
			strings
				.iter()
				.copied()
				.find(|s| *s != slug && !s.starts_with("http://") && !s.starts_with("https://"))
		})?;
	Some((slug.into(), title.into()))
}

fn parse_home_featured(flight: &Flight) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	for array in flight.all_arrays() {
		let records: Vec<&Map<String, Value>> = array.iter().filter_map(Value::as_object).collect();
		if !records
			.first()
			.is_some_and(|r| r.contains_key("protagonist") || r.contains_key("banner"))
		{
			continue;
		}
		let entries: Vec<Manga> = records
			.into_iter()
			.filter_map(|record| {
				let (slug, title) = extract_home_entry(flight, record)?;
				if !seen.insert(slug.clone()) {
					return None;
				}
				Some(Manga {
					url: Some(format!("{BASE_URL}/comic/{slug}")),
					key: slug,
					title,
					cover: image_url(
						record
							.get("protagonist")
							.or_else(|| record.get("banner"))
							.or_else(|| record.get("thumbnail"))
							.and_then(Value::as_str)
							.and_then(|s| flight.resolve_str(s)),
					),
					description: record
						.get("description")
						.and_then(Value::as_str)
						.and_then(|s| flight.resolve_str(s))
						.map(strip_html)
						.filter(|d| !d.is_empty()),
					content_rating: record
						.get("badge")
						.and_then(Value::as_str)
						.and_then(|s| flight.resolve_str(s))
						.map(rating_of)
						.unwrap_or(ContentRating::Safe),
					..Default::default()
				})
			})
			.collect();
		if !entries.is_empty() {
			return entries;
		}
	}
	Vec::new()
}

fn parse_card_anchor(anchor: &Element, title_override: Option<String>) -> Option<Manga> {
	let (slug, chapter) = parse_comic_href(&anchor.attr("href")?)?;
	if chapter.is_some() {
		return None;
	}
	let title = title_override
		.or_else(|| {
			anchor
				.select_first("h3, span[class*='line-clamp']")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
		})
		.filter(|t| !t.is_empty())?;
	let cover = anchor
		.select_first("img")
		.and_then(|img| img.attr("src").or_else(|| img.attr("data-src")))
		.and_then(|s| image_url(Some(&s)));
	let content_rating = if anchor
		.select("span")
		.into_iter()
		.flatten()
		.filter_map(|span| span.text())
		.any(|label| rating_of(&clean(&label)) == ContentRating::NSFW)
	{
		ContentRating::NSFW
	} else {
		ContentRating::Safe
	};
	Some(Manga {
		url: Some(format!("{BASE_URL}/comic/{slug}")),
		key: slug,
		title,
		cover,
		content_rating,
		..Default::default()
	})
}

fn parse_home_trending_week(doc: &Document, flight: &Flight) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	if let Some(items) = flight
		.find(&["weekRes"])
		.and_then(|props| props.get("weekRes"))
		.and_then(Value::as_array)
	{
		let entries: Vec<Manga> = items
			.iter()
			.filter_map(Value::as_object)
			.filter_map(|record| {
				let (slug, title) = extract_home_entry(flight, record)?;
				if !seen.insert(slug.clone()) {
					return None;
				}
				Some(Manga {
					url: Some(format!("{BASE_URL}/comic/{slug}")),
					key: slug,
					title,
					cover: image_url(
						record
							.get("thumbnail")
							.and_then(Value::as_str)
							.and_then(|s| flight.resolve_str(s)),
					),
					content_rating: record
						.get("badge")
						.and_then(Value::as_str)
						.and_then(|s| flight.resolve_str(s))
						.map(rating_of)
						.unwrap_or(ContentRating::Safe),
					..Default::default()
				})
			})
			.collect();
		if !entries.is_empty() {
			return entries;
		}
	}
	doc.select("section[aria-labelledby='trending-title'] ol > li > a[href]")
		.into_iter()
		.flatten()
		.filter_map(|anchor| {
			let manga = parse_card_anchor(&anchor, None)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect()
}

fn is_badge(label: &str) -> bool {
	["new", "premium", "up next", "read"]
		.iter()
		.any(|badge| label.eq_ignore_ascii_case(badge))
}

fn parse_chapter_anchor(
	anchor: &Element,
	series: &str,
	chapter_slug: String,
	position: Option<f32>,
	include_url: bool,
) -> Chapter {
	let mut labels: Vec<String> = Vec::new();
	let mut premium = false;
	for span in anchor.select("span").into_iter().flatten() {
		let label = span.own_text().map(|t| clean(&t)).unwrap_or_default();
		if label.eq_ignore_ascii_case("premium") {
			premium = true;
		}
		if !label.is_empty() && !is_badge(&label) && !span.has_class("sr-only") {
			labels.push(label);
		}
	}
	let name = labels
		.first()
		.cloned()
		.unwrap_or_else(|| chapter_slug.clone());
	let number = first_number(&name).or(position);
	let subtitle = labels
		.get(1)
		.filter(|s| {
			anchor
				.select_first("time")
				.and_then(|t| t.text())
				.is_none_or(|dt| !dt.contains(s.as_str()))
		})
		.and_then(|s| strip_chapter_label(s, number));
	let title = subtitle.or_else(|| strip_chapter_label(&name, number));
	let date_uploaded = anchor
		.select_first("time[datetime]")
		.and_then(|el| el.attr("datetime"))
		.and_then(|dt| parse_iso_utc(Some(&dt)));
	Chapter {
		url: include_url.then(|| format!("{BASE_URL}/comic/{series}/{chapter_slug}")),
		key: chapter_slug,
		title,
		chapter_number: number,
		date_uploaded,
		locked: premium,
		..Default::default()
	}
}

fn parse_home_latest(doc: &Document, show_locked: bool) -> Vec<MangaWithChapter> {
	let Some(section) = doc.select("section").into_iter().flatten().find(|s| {
		s.select_first("#latest-updates-title").is_some()
			|| s.select_first("h2")
				.and_then(|h| h.text())
				.is_some_and(|t| clean(&t).eq_ignore_ascii_case("latest updates"))
	}) else {
		return Vec::new();
	};
	let mut seen = BTreeSet::new();
	section
		.select("article")
		.into_iter()
		.flatten()
		.filter_map(|article| {
			if article.attr("data-sponsored").is_some() {
				return None;
			}
			let cover_link = article.select_first("a[href]")?;
			let title = article
				.select_first("h3 a[href]")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			let manga = parse_card_anchor(&cover_link, title)?;
			let mut fallback = None;
			let mut picked = None;
			for anchor in article.select("ul a[href]").into_iter().flatten() {
				let Some(href) = anchor.attr("href") else {
					continue;
				};
				if let Some((series, Some(ch_slug))) = parse_comic_href(&href)
					&& series == manga.key
				{
					let ch = parse_chapter_anchor(&anchor, &series, ch_slug, None, false);
					if !ch.locked {
						picked = Some(ch);
						break;
					} else if fallback.is_none() {
						fallback = Some(ch);
					}
				}
			}
			let chapter = picked.or_else(|| show_locked.then_some(fallback).flatten())?;
			seen.insert(manga.key.clone())
				.then_some(MangaWithChapter { manga, chapter })
		})
		.collect()
}

fn parse_home_new_series(doc: &Document) -> Vec<Manga> {
	let Some(section) = doc.select("section").into_iter().flatten().find(|s| {
		s.select_first("h2")
			.and_then(|h| h.text())
			.is_some_and(|t| clean(&t).eq_ignore_ascii_case("new series"))
	}) else {
		return Vec::new();
	};
	let mut seen = BTreeSet::new();
	section
		.select("a[href]")
		.into_iter()
		.flatten()
		.filter_map(|anchor| {
			anchor.select_first("img")?;
			let manga = parse_card_anchor(&anchor, None)?;
			seen.insert(manga.key.clone()).then_some(manga)
		})
		.collect()
}

fn listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}

pub fn build_home_layout(doc: &Document, show_locked: bool) -> HomeLayout {
	let flight = Flight::from_document(doc);
	let mut components = Vec::new();

	let featured = parse_home_featured(&flight);
	if !featured.is_empty() {
		components.push(HomeComponent {
			title: Some("Featured".into()),
			subtitle: None,
			value: HomeComponentValue::BigScroller {
				entries: featured,
				auto_scroll_interval: Some(6.0),
			},
		});
	}

	let trending: Vec<Link> = parse_home_trending_week(doc, &flight)
		.into_iter()
		.map(Into::into)
		.collect();
	if !trending.is_empty() {
		components.push(HomeComponent {
			title: Some("Trending This Week".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: trending,
				listing: Some(listing("views", "Trending")),
			},
		});
	}

	let latest = parse_home_latest(doc, show_locked);
	if !latest.is_empty() {
		components.push(HomeComponent {
			title: Some("Latest Updates".into()),
			subtitle: None,
			value: HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: latest,
				listing: Some(listing("updated", "Latest Updates")),
			},
		});
	}

	let new_series: Vec<Link> = parse_home_new_series(doc)
		.into_iter()
		.map(Into::into)
		.collect();
	if !new_series.is_empty() {
		components.push(HomeComponent {
			title: Some("New Series".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: new_series,
				listing: Some(listing("created", "New Series")),
			},
		});
	}

	HomeLayout { components }
}

pub fn parse_series_details(doc: &Document, manga: &mut Manga) {
	if let Some(title) = doc
		.select_first("h1#series-title, main h1, h1")
		.and_then(|el| el.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())
	{
		manga.title = title;
	}
	manga.cover = doc
		.select_first("meta[property='og:image']")
		.and_then(|el| el.attr("content"))
		.and_then(|s| image_url(Some(&s)))
		.or_else(|| manga.cover.take());

	if let Some(synopsis) = doc.select_first("#series-synopsis-text") {
		let paragraphs: Vec<String> = synopsis
			.select("p")
			.into_iter()
			.flatten()
			.filter_map(|p| p.text())
			.map(|t| clean(&t))
			.filter(|t| !t.is_empty())
			.collect();
		let text = if !paragraphs.is_empty() {
			paragraphs.join("\n\n")
		} else {
			synopsis.text().map(|t| strip_html(&t)).unwrap_or_default()
		};
		if !text.is_empty() {
			manga.description = Some(text.escape_markdown());
		}
	}

	for entry in doc.select("dl > div").into_iter().flatten() {
		if entry
			.select_first("dt")
			.and_then(|el| el.text())
			.is_some_and(|t| clean(&t).eq_ignore_ascii_case("author"))
			&& let Some(val) = entry
				.select_first("dd")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
		{
			let mut authors: Vec<String> = val
				.split([',', '|', ';'])
				.map(clean)
				.filter(|s| !s.is_empty())
				.collect();
			authors.sort_unstable();
			authors.dedup();
			manga.authors = (!authors.is_empty()).then_some(authors);
		}
	}

	for stat in doc
		.select("ul[aria-label='Series stats'] > li")
		.into_iter()
		.flatten()
	{
		if let Some(text) = stat.text().map(|t| clean(&t)) {
			let parsed = parse_status(Some(&text));
			if parsed != MangaStatus::Unknown {
				manga.status = parsed;
				break;
			}
		}
	}

	let mut tags: Vec<String> = doc
		.select("ul[aria-label='Genres'] > li")
		.into_iter()
		.flatten()
		.filter_map(|el| el.text())
		.map(|t| clean(&t))
		.filter(|t| !t.is_empty())
		.collect();
	tags.sort_unstable();
	tags.dedup();
	let mut content_rating = ContentRating::Safe;
	for tag in &tags {
		match rating_of(tag) {
			ContentRating::NSFW => {
				content_rating = ContentRating::NSFW;
				break;
			}
			ContentRating::Suggestive => content_rating = ContentRating::Suggestive,
			_ => {}
		}
	}
	manga.content_rating = content_rating;
	manga.tags = (!tags.is_empty()).then_some(tags);
	manga.viewer = Viewer::Webtoon;
	manga.url = Some(format!("{BASE_URL}/comic/{}", manga.key));
}

fn chapter_positions(doc: &Document) -> BTreeMap<String, f32> {
	let mut map = BTreeMap::new();
	let flight = Flight::from_document(doc);
	let Some(seasons) = flight
		.find(&["seriesData"])
		.and_then(|props| props.get("seriesData"))
		.and_then(Value::as_object)
		.and_then(|series| series.values().find_map(Value::as_array))
	else {
		return map;
	};
	let known = ["index", "chapter_title", "chapter_thumbnail", "created_at"];
	for season in seasons.iter().filter_map(Value::as_object) {
		let Some(chapters) = season.values().find_map(Value::as_array) else {
			continue;
		};
		let records: Vec<&Map<String, Value>> =
			chapters.iter().filter_map(Value::as_object).collect();
		let Some(fields) = Classified::of(&records, &known) else {
			continue;
		};
		for record in records {
			let Some(slug) = record.get(&fields.slug).and_then(Value::as_str) else {
				continue;
			};
			let pos = match record.get("index") {
				Some(Value::String(s)) => s.trim().parse::<f32>().ok(),
				Some(Value::Number(n)) => n.as_f64().map(|f| f as f32),
				_ => None,
			};
			if let Some(p) = pos.filter(|p| p.is_finite()) {
				map.insert(slug.into(), p);
			}
		}
	}
	map
}

pub fn parse_series_chapters(doc: &Document, slug: &str, show_locked: bool) -> Vec<Chapter> {
	let positions = chapter_positions(doc);
	let mut seen = BTreeSet::new();
	doc.select("#chapter-list > li")
		.into_iter()
		.flatten()
		.filter_map(|item| {
			let anchor = item.select_first("a[href]")?;
			let (owner, Some(ch_slug)) = parse_comic_href(&anchor.attr("href")?)? else {
				return None;
			};
			if owner != slug || !seen.insert(ch_slug.clone()) {
				return None;
			}
			let pos = positions.get(&ch_slug).copied();
			let chapter = parse_chapter_anchor(&anchor, slug, ch_slug, pos, true);
			(show_locked || !chapter.locked).then_some(chapter)
		})
		.collect()
}

pub fn decode_reader_tokens(tokens_list: &[String], seed: &str) -> Option<Vec<String>> {
	let mut urls = Vec::with_capacity(tokens_list.len());
	for (index, token) in tokens_list.iter().enumerate() {
		let url = if token.starts_with("https://") || token.starts_with("http://") {
			token.clone()
		} else {
			tokens::decode(token, index, seed)?
		};
		if !(url.starts_with("https://") || url.starts_with("http://")) {
			return None;
		}
		urls.push(url);
	}
	Some(urls)
}

pub fn extract_reader_tokens(doc: &Document) -> Option<ReaderData> {
	let flight = Flight::from_document(doc);
	let found = flight.all(&["layoutHints"]);
	let [props] = found.as_slice() else {
		return None;
	};
	let hints = props.get("layoutHints")?.as_array()?;
	let mut lists = props.iter().filter_map(|(key, value)| match value {
		Value::Array(items)
			if key != "layoutHints"
				&& items.len() == hints.len()
				&& items.iter().all(Value::is_string) =>
		{
			Some(items)
		}
		_ => None,
	});
	let (Some(list), None) = (lists.next(), lists.next()) else {
		return None;
	};
	let tokens: Vec<String> = list
		.iter()
		.filter_map(Value::as_str)
		.map(String::from)
		.collect();
	let script = flight.reference_of(props).and_then(|ref_id| {
		let parts = flight.module(ref_id)?.as_array()?;
		let module = parts.first()?.as_u64()?;
		let chunk = parts.get(1)?.as_array()?.last()?.as_str()?;
		(chunk.starts_with("static/chunks/") && chunk.ends_with(".js") && !chunk.contains(".."))
			.then(|| (module, chunk.into()))
	});
	Some(ReaderData { tokens, script })
}
