#![no_std]
mod rsc;
#[cfg(test)]
mod test;
mod tokens;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, ImageRequestProvider, Link, Listing, ListingKind,
	ListingProvider, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Page, PageContent,
	PageContext, Result, Source, Viewer,
	alloc::{String, Vec, collections::BTreeMap, string::ToString},
	imports::{
		html::{Document, Element},
		net::Request,
		std::parse_date_with_options,
	},
	prelude::*,
};
use rsc::{Classified, Flight, slug_like};
use serde_json::{Map, Value};

const DOMAIN: &str = "https://templetoons.com";
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

struct CatalogItem {
	slug: String,
	title: String,
	cover: Option<String>,
	badge: Option<String>,
	status: Option<String>,
	alternative_names: Option<String>,
	created_at: Option<i64>,
	updated_at: Option<i64>,
	views: u64,
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
			format!("{DOMAIN}{path}")
		} else {
			format!("{DOMAIN}/{path}")
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
	if raw.len() >= 19 {
		let formatted = raw[..19].replace('T', " ");
		parse_date_with_options(formatted, "yyyy-MM-dd HH:mm:ss", "en_US_POSIX", "UTC")
	} else if raw.len() >= 10 {
		parse_date_with_options(&raw[..10], "yyyy-MM-dd", "en_US_POSIX", "UTC")
	} else {
		None
	}
}

fn chapter_number_from_name(name: &str) -> Option<f32> {
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

fn parse_comic_href(href: &str) -> Option<(String, Option<String>)> {
	let after = href.split("/comic/").nth(1)?;
	let mut parts = after.split(['/', '?', '#']);
	let slug = parts.next().filter(|s| !s.is_empty())?;
	let chapter = parts.next().filter(|s| !s.is_empty());
	Some((slug.to_string(), chapter.map(String::from)))
}

fn is_challenge_page(doc: &Document) -> bool {
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
	doc.select_first("script[src*='challenges.cloudflare.com'], .cf-turnstile")
		.is_some()
}

fn fetch_doc(url: &str) -> Result<Document> {
	let response = Request::get(url)?
		.header("Referer", &format!("{DOMAIN}/"))
		.send()?;
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
			.or_else(|| n.as_f64().map(|f| f as u64))
			.unwrap_or(0),
		Some(Value::String(s)) => s.trim().parse().unwrap_or(0),
		_ => 0,
	}
}

fn parse_catalog(doc: &Document) -> Vec<CatalogItem> {
	let flight = Flight::from_document(doc);
	let mut best: Vec<CatalogItem> = Vec::new();
	for array in flight.all_arrays() {
		let records: Vec<&Map<String, Value>> = array.iter().filter_map(Value::as_object).collect();
		if records.len() <= best.len() {
			continue;
		}
		if !records.first().is_some_and(|r| r.contains_key("thumbnail")) {
			continue;
		}
		let Some(fields) = Classified::of(&records, &CATALOG_KNOWN) else {
			continue;
		};
		let parsed: Vec<CatalogItem> = records
			.into_iter()
			.filter_map(|record| {
				let slug = record.get(&fields.slug)?.as_str()?.trim();
				let title = record.get(&fields.text)?.as_str()?.trim();
				if slug.is_empty() || title.is_empty() {
					return None;
				}
				Some(CatalogItem {
					slug: slug.to_string(),
					title: title.to_string(),
					cover: image_url(record.get("thumbnail").and_then(Value::as_str)),
					badge: record
						.get("badge")
						.and_then(Value::as_str)
						.map(|s| s.trim().to_string()),
					status: record
						.get("status")
						.and_then(Value::as_str)
						.map(|s| s.trim().to_string()),
					alternative_names: record
						.get("alternative_names")
						.and_then(Value::as_str)
						.map(String::from),
					created_at: parse_iso_utc(record.get("created_at").and_then(Value::as_str)),
					updated_at: parse_iso_utc(record.get("update_chapter").and_then(Value::as_str)),
					views: parse_views(record.get("total_views")),
				})
			})
			.collect();
		if parsed.len() > best.len() {
			best = parsed;
		}
	}
	best
}

fn catalog_to_manga(item: CatalogItem) -> Manga {
	let content_rating = item
		.badge
		.as_deref()
		.map(rating_of)
		.unwrap_or(ContentRating::Safe);
	Manga {
		url: Some(format!("{DOMAIN}/comic/{}", item.slug)),
		key: item.slug,
		title: item.title,
		cover: item.cover,
		status: parse_status(item.status.as_deref()),
		content_rating,
		viewer: Viewer::Webtoon,
		..Default::default()
	}
}

fn sort_catalog(items: &mut [CatalogItem], sort: i32) {
	match sort {
		1 => items.sort_unstable_by_key(|a| core::cmp::Reverse(a.updated_at)),
		2 => items.sort_unstable_by_key(|a| core::cmp::Reverse(a.created_at)),
		_ => items.sort_unstable_by_key(|a| core::cmp::Reverse(a.views)),
	}
}

fn page_catalog(items: Vec<CatalogItem>, page: i32) -> MangaPageResult {
	let start = (page.max(1) as usize - 1) * PAGE_SIZE;
	let has_next_page = items.len() > start + PAGE_SIZE;
	MangaPageResult {
		entries: items
			.into_iter()
			.skip(start)
			.take(PAGE_SIZE)
			.map(catalog_to_manga)
			.collect(),
		has_next_page,
	}
}

fn extract_home_entry(record: &Map<String, Value>) -> Option<(String, String)> {
	let strings: Vec<&str> = record
		.iter()
		.filter(|(k, _)| !HOME_KNOWN.contains(&k.as_str()))
		.filter_map(|(_, v)| v.as_str().map(str::trim).filter(|s| !s.is_empty()))
		.collect();
	let slug = record
		.get("series_slug")
		.and_then(Value::as_str)
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.or_else(|| strings.iter().copied().find(|s| slug_like(s)))?;
	let title = record
		.get("title")
		.and_then(Value::as_str)
		.map(str::trim)
		.filter(|s| !s.is_empty())
		.or_else(|| {
			strings
				.iter()
				.copied()
				.find(|s| *s != slug && !s.starts_with("http://") && !s.starts_with("https://"))
		})?;
	Some((slug.to_string(), title.to_string()))
}

fn parse_home_featured(flight: &Flight) -> Vec<Manga> {
	let mut entries = Vec::new();
	let mut seen = Vec::new();
	for array in flight.all_arrays() {
		let records: Vec<&Map<String, Value>> = array.iter().filter_map(Value::as_object).collect();
		if !records
			.first()
			.is_some_and(|r| r.contains_key("protagonist") || r.contains_key("banner"))
		{
			continue;
		}
		for record in records {
			let Some((slug, title)) = extract_home_entry(record) else {
				continue;
			};
			if seen.contains(&slug) {
				continue;
			}
			let cover = image_url(
				record
					.get("protagonist")
					.or_else(|| record.get("banner"))
					.or_else(|| record.get("thumbnail"))
					.and_then(Value::as_str),
			);
			let description = record
				.get("description")
				.and_then(Value::as_str)
				.map(strip_html)
				.filter(|d| !d.is_empty());
			let content_rating = record
				.get("badge")
				.and_then(Value::as_str)
				.map(rating_of)
				.unwrap_or(ContentRating::Safe);
			seen.push(slug.clone());
			entries.push(Manga {
				url: Some(format!("{DOMAIN}/comic/{slug}")),
				key: slug,
				title,
				cover,
				description,
				content_rating,
				viewer: Viewer::Webtoon,
				..Default::default()
			});
		}
		if !entries.is_empty() {
			break;
		}
	}
	entries
}

fn parse_card_anchor(anchor: &Element, title_override: Option<String>) -> Option<Manga> {
	let href = anchor.attr("href")?;
	let (slug, chapter) = parse_comic_href(&href)?;
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
	let mut content_rating = ContentRating::Safe;
	if let Some(spans) = anchor.select("span") {
		for span in spans {
			if let Some(label) = span.text().map(|t| clean(&t)) {
				let r = rating_of(&label);
				if r == ContentRating::NSFW {
					content_rating = ContentRating::NSFW;
					break;
				}
			}
		}
	}
	Some(Manga {
		url: Some(format!("{DOMAIN}/comic/{slug}")),
		key: slug,
		title,
		cover,
		content_rating,
		viewer: Viewer::Webtoon,
		..Default::default()
	})
}

fn parse_home_trending_week(doc: &Document, flight: &Flight) -> Vec<Manga> {
	let mut entries = Vec::new();
	let mut seen = Vec::new();
	if let Some(props) = flight.find(&["weekRes"])
		&& let Some(items) = props.get("weekRes").and_then(Value::as_array)
	{
		for record in items.iter().filter_map(Value::as_object) {
			let Some((slug, title)) = extract_home_entry(record) else {
				continue;
			};
			if seen.contains(&slug) {
				continue;
			}
			let cover = image_url(record.get("thumbnail").and_then(Value::as_str));
			let content_rating = record
				.get("badge")
				.and_then(Value::as_str)
				.map(rating_of)
				.unwrap_or(ContentRating::Safe);
			seen.push(slug.clone());
			entries.push(Manga {
				url: Some(format!("{DOMAIN}/comic/{slug}")),
				key: slug,
				title,
				cover,
				content_rating,
				viewer: Viewer::Webtoon,
				..Default::default()
			});
		}
	}
	if !entries.is_empty() {
		return entries;
	}
	if let Some(anchors) = doc.select("section[aria-labelledby='trending-title'] ol > li > a[href]")
	{
		for anchor in anchors {
			if let Some(manga) = parse_card_anchor(&anchor, None)
				&& !seen.contains(&manga.key)
			{
				seen.push(manga.key.clone());
				entries.push(manga);
			}
		}
	}
	entries
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
) -> Chapter {
	let mut labels: Vec<String> = Vec::new();
	let mut premium = false;
	if let Some(spans) = anchor.select("span") {
		for span in spans {
			let label = span.own_text().map(|t| clean(&t)).unwrap_or_default();
			if label.eq_ignore_ascii_case("premium") {
				premium = true;
			}
			if !label.is_empty() && !is_badge(&label) && !span.has_class("sr-only") {
				labels.push(label);
			}
		}
	}
	let name = labels
		.first()
		.cloned()
		.unwrap_or_else(|| chapter_slug.clone());
	let subtitle = labels
		.get(1)
		.filter(|s| {
			anchor
				.select_first("time")
				.and_then(|t| t.text())
				.is_none_or(|dt| !dt.contains(s.as_str()))
		})
		.cloned();
	let number = chapter_number_from_name(&name).or(position);
	let date_uploaded = anchor
		.select_first("time[datetime]")
		.and_then(|el| el.attr("datetime"))
		.and_then(|dt| parse_iso_utc(Some(&dt)));
	let title = match subtitle {
		Some(sub) => Some(sub),
		None => {
			let stripped = name
				.trim()
				.strip_prefix("Chapter")
				.or_else(|| name.trim().strip_prefix("chapter"))
				.map(|rest| {
					let rest = rest.trim_start();
					let after_num =
						rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
					after_num
						.trim_start_matches(|c: char| {
							c.is_whitespace() || matches!(c, ':' | '-' | '–' | '—')
						})
						.trim()
				});
			match stripped {
				Some("") => None,
				Some(rest) => Some(rest.to_string()),
				None => (!name.is_empty()).then_some(name),
			}
		}
	};

	Chapter {
		url: Some(format!("{DOMAIN}/comic/{series}/{chapter_slug}")),
		key: chapter_slug,
		title,
		chapter_number: number,
		date_uploaded,
		locked: premium,
		..Default::default()
	}
}

fn parse_home_latest(doc: &Document) -> Vec<MangaWithChapter> {
	let mut updates: Vec<MangaWithChapter> = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	let Some(sections) = doc.select("section") else {
		return updates;
	};
	let Some(section) = sections.into_iter().find(|s| {
		s.select_first("#latest-updates-title").is_some()
			|| s.select_first("h2")
				.and_then(|h| h.text())
				.is_some_and(|t| clean(&t).eq_ignore_ascii_case("latest updates"))
	}) else {
		return updates;
	};
	let Some(articles) = section.select("article") else {
		return updates;
	};
	for article in articles {
		if article.attr("data-sponsored").is_some() {
			continue;
		}
		let Some(cover_link) = article.select_first("a[href]") else {
			continue;
		};
		let title = article
			.select_first("h3 a[href]")
			.and_then(|el| el.text())
			.map(|t| clean(&t))
			.filter(|t| !t.is_empty());
		let Some(manga) = parse_card_anchor(&cover_link, title) else {
			continue;
		};
		if seen.contains(&manga.key) {
			continue;
		}
		let mut picked: Option<Chapter> = None;
		if let Some(anchors) = article.select("ul a[href]") {
			let mut fallback: Option<Chapter> = None;
			for anchor in anchors {
				let Some(href) = anchor.attr("href") else {
					continue;
				};
				if let Some((series, Some(ch_slug))) = parse_comic_href(&href)
					&& series == manga.key
				{
					let ch = parse_chapter_anchor(&anchor, &series, ch_slug, None);
					if !ch.locked {
						picked = Some(ch);
						break;
					} else if fallback.is_none() {
						fallback = Some(ch);
					}
				}
			}
			if picked.is_none() {
				picked = fallback;
			}
		}
		if let Some(chapter) = picked {
			seen.push(manga.key.clone());
			updates.push(MangaWithChapter { manga, chapter });
		}
	}
	updates
}

fn parse_home_new_series(doc: &Document) -> Vec<Manga> {
	let mut entries: Vec<Manga> = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	let Some(sections) = doc.select("section") else {
		return entries;
	};
	let Some(section) = sections.into_iter().find(|s| {
		s.select_first("h2")
			.and_then(|h| h.text())
			.is_some_and(|t| clean(&t).eq_ignore_ascii_case("new series"))
	}) else {
		return entries;
	};
	if let Some(anchors) = section.select("a[href]") {
		for anchor in anchors {
			if anchor.select_first("img").is_none() {
				continue;
			}
			if let Some(manga) = parse_card_anchor(&anchor, None)
				&& !seen.contains(&manga.key)
			{
				seen.push(manga.key.clone());
				entries.push(manga);
			}
		}
	}
	entries
}

fn make_listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}

fn build_home_layout(doc: &Document) -> HomeLayout {
	let flight = Flight::from_document(doc);
	let mut components: Vec<HomeComponent> = Vec::new();

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
				listing: Some(make_listing("views", "Most Viewed")),
			},
		});
	}

	let latest = parse_home_latest(doc);
	if !latest.is_empty() {
		components.push(HomeComponent {
			title: Some("Latest Updates".into()),
			subtitle: None,
			value: HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: latest,
				listing: Some(make_listing("updated", "Latest Updates")),
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
				listing: Some(make_listing("created", "Newest")),
			},
		});
	}

	HomeLayout { components }
}

fn parse_series_details(doc: &Document, manga: &mut Manga) {
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
			.map(|ps| {
				ps.filter_map(|p| p.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
					.collect()
			})
			.unwrap_or_default();
		let text = if !paragraphs.is_empty() {
			paragraphs.join("\n\n")
		} else {
			synopsis.text().map(|t| strip_html(&t)).unwrap_or_default()
		};
		if !text.is_empty() {
			manga.description = Some(text);
		}
	}

	if let Some(entries) = doc.select("dl > div") {
		for entry in entries {
			let label = entry
				.select_first("dt")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.unwrap_or_default();
			if label.eq_ignore_ascii_case("author")
				&& let Some(val) = entry
					.select_first("dd")
					.and_then(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
			{
				manga.authors = Some(
					val.split([',', '|', ';'])
						.map(clean)
						.filter(|s| !s.is_empty())
						.collect(),
				);
			}
		}
	}

	if let Some(stats) = doc.select("ul[aria-label='Series stats'] > li") {
		for stat in stats {
			if let Some(text) = stat.text().map(|t| clean(&t)) {
				let parsed = parse_status(Some(&text));
				if parsed != MangaStatus::Unknown {
					manga.status = parsed;
					break;
				}
			}
		}
	}

	let mut tags: Vec<String> = Vec::new();
	if let Some(genres) = doc.select("ul[aria-label='Genres'] > li") {
		for genre in genres {
			if let Some(tag) = genre.text().map(|t| clean(&t)).filter(|t| !t.is_empty())
				&& !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag))
			{
				tags.push(tag);
			}
		}
	}
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
	manga.url = Some(format!("{DOMAIN}/comic/{}", manga.key));
}

fn chapter_positions(doc: &Document) -> BTreeMap<String, f32> {
	let mut map = BTreeMap::new();
	let flight = Flight::from_document(doc);
	let Some(props) = flight.find(&["seriesData"]) else {
		return map;
	};
	let Some(Value::Object(series)) = props.get("seriesData") else {
		return map;
	};
	let Some(seasons) = series.values().find_map(Value::as_array) else {
		return map;
	};
	let known = ["index", "chapter_title", "chapter_thumbnail", "created_at"];
	for season in seasons.iter().filter_map(Value::as_object) {
		let Some(Value::Array(chapters)) = season.values().find(|v| v.is_array()) else {
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
				map.insert(slug.to_string(), p);
			}
		}
	}
	map
}

fn parse_series_chapters(doc: &Document, slug: &str) -> Vec<Chapter> {
	let positions = chapter_positions(doc);
	let mut chapters: Vec<Chapter> = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	if let Some(items) = doc.select("#chapter-list > li") {
		for item in items {
			let Some(anchor) = item.select_first("a[href]") else {
				continue;
			};
			let Some(href) = anchor.attr("href") else {
				continue;
			};
			let Some((owner, Some(ch_slug))) = parse_comic_href(&href) else {
				continue;
			};
			if owner != slug || seen.contains(&ch_slug) {
				continue;
			}
			seen.push(ch_slug.clone());
			let pos = positions.get(&ch_slug).copied();
			chapters.push(parse_chapter_anchor(&anchor, slug, ch_slug, pos));
		}
	}
	chapters
}

fn decode_reader_tokens(tokens_list: &[String], seed: &str) -> Option<Vec<String>> {
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

struct ReaderData {
	tokens: Vec<String>,
	script: Option<(u64, String)>,
}

fn extract_reader_tokens(doc: &Document) -> Option<ReaderData> {
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
		let chunks = parts.get(1)?.as_array()?;
		let chunk = chunks.last()?.as_str()?;
		(chunk.starts_with("static/chunks/") && chunk.ends_with(".js") && !chunk.contains(".."))
			.then(|| (module, chunk.to_string()))
	});
	Some(ReaderData { tokens, script })
}

struct TempleScan;

impl Source for TempleScan {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let query = query.unwrap_or_default().trim().to_ascii_lowercase();
		let mut sort = 0;
		let mut status = String::new();
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => sort = index,
				FilterValue::Select { id, value } if id == "status" && !value.is_empty() => {
					status = value;
				}
				_ => {}
			}
		}
		let doc = fetch_doc(&format!("{DOMAIN}/comics"))?;
		let mut items = parse_catalog(&doc);
		if items.is_empty() {
			bail!("No series catalog found on Temple Scan");
		}
		items.retain(|series| {
			if !query.is_empty() {
				let title = series.title.to_ascii_lowercase();
				let alt = series
					.alternative_names
					.as_deref()
					.unwrap_or("")
					.to_ascii_lowercase();
				if !title.contains(&query) && !alt.contains(&query) {
					return false;
				}
			}
			if !status.is_empty()
				&& !series
					.status
					.as_deref()
					.unwrap_or("")
					.eq_ignore_ascii_case(&status)
			{
				return false;
			}
			true
		});
		sort_catalog(&mut items, sort);
		Ok(page_catalog(items, page))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let slug = manga.key.clone();
		let doc = fetch_doc(&format!("{DOMAIN}/comic/{slug}"))?;
		if needs_details {
			parse_series_details(&doc, &mut manga);
		}
		if needs_chapters {
			manga.chapters = Some(parse_series_chapters(&doc, &slug));
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if chapter.locked {
			bail!("This chapter is early access for subscribers on Temple Scan");
		}
		let doc = fetch_doc(&format!("{DOMAIN}/comic/{}/{}", manga.key, chapter.key))?;
		if doc.select("main p").is_some_and(|ps| {
			ps.into_iter().any(|p| {
				p.text()
					.is_some_and(|t| t.to_ascii_lowercase().contains("chapter is early access"))
			})
		}) {
			bail!("This chapter is early access for subscribers on Temple Scan");
		}

		let Some(reader) = extract_reader_tokens(&doc) else {
			bail!("No pages found for chapter {}", chapter.key);
		};
		let urls = match decode_reader_tokens(&reader.tokens, tokens::SEED) {
			Some(urls) => urls,
			None => {
				let Some((module, chunk)) = reader.script else {
					bail!("Temple Scan changed its reader encoding");
				};
				let body = Request::get(format!("{DOMAIN}/_next/{chunk}"))?
					.header("Referer", &format!("{DOMAIN}/"))
					.string()?;
				let Some(seed) = tokens::seed_from_chunk(&body, module) else {
					bail!("Temple Scan changed its reader encoding");
				};
				decode_reader_tokens(&reader.tokens, &seed)
					.ok_or_else(|| error!("Temple Scan changed its reader encoding"))?
			}
		};
		if urls.is_empty() {
			bail!("No pages found for chapter {}", chapter.key);
		}
		Ok(urls
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

impl Home for TempleScan {
	fn get_home(&self) -> Result<HomeLayout> {
		let doc = fetch_doc(&format!("{DOMAIN}/"))?;
		Ok(build_home_layout(&doc))
	}
}

impl ListingProvider for TempleScan {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let sort = match listing.id.as_str() {
			"views" => 0,
			"updated" => 1,
			"created" => 2,
			_ => return Err(error!("Unknown listing")),
		};
		let doc = fetch_doc(&format!("{DOMAIN}/comics"))?;
		let mut items = parse_catalog(&doc);
		if items.is_empty() {
			bail!("No series catalog found on Temple Scan");
		}
		sort_catalog(&mut items, sort);
		Ok(page_catalog(items, page))
	}
}

impl ImageRequestProvider for TempleScan {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{DOMAIN}/")))
	}
}

impl DeepLinkHandler for TempleScan {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some((slug, chapter)) = parse_comic_href(&url) else {
			return Ok(None);
		};
		if let Some(ch) = chapter {
			return Ok(Some(DeepLinkResult::Chapter {
				manga_key: slug,
				key: ch,
			}));
		}
		Ok(Some(DeepLinkResult::Manga { key: slug }))
	}
}

register_source!(
	TempleScan,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
