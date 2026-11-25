use aidoku::{
	ContentRating, FilterItem, FilterValue, Listing, ListingKind, Manga, MangaPageResult, Result,
	Viewer,
	alloc::{String, Vec, vec},
	helpers::{string::StripPrefixOrSelf, uri::QueryParameters},
	imports::{html::Element, net::Request, std::current_date},
	prelude::*,
};
use madara::{
	Params,
	helpers::{self, ElementImageAttr},
};

pub const BASE_URL: &str = "https://aquamanga.io";

pub const SORTS: [&str; 5] = ["latest", "popular", "views", "new-manga", "alphabet"];

const BROWSE_GENRES: &[(&str, &str)] = &[
	("Manga", "manga"),
	("Manhwa", "manhwa"),
	("Manhua", "manhua"),
	("Action", "action"),
	("Adventure", "adventure"),
	("Comedy", "comedy"),
	("Drama", "drama"),
	("Fantasy", "fantasy"),
	("Romance", "romance"),
	("Isekai", "isekai"),
	("Supernatural", "supernatural"),
	("Horror", "horror"),
	("Mystery", "mystery"),
	("Martial Arts", "martial-arts"),
	("Regression", "regression"),
	("Reincarnation", "reincarnation"),
	("Survival", "survival"),
	("System", "system"),
];

#[derive(Clone, Copy)]
pub struct ArchiveQuery<'a> {
	pub page: i32,
	pub query: Option<&'a str>,
	pub sort: &'a str,
	pub status: Option<&'a str>,
	pub genre: Option<&'a str>,
	pub ch_min: Option<f32>,
	pub ch_max: Option<f32>,
}

pub fn fetch_archive(q: ArchiveQuery<'_>) -> Result<MangaPageResult> {
	let mut qs = QueryParameters::new();
	if !q.sort.is_empty() {
		qs.push_encoded("m_orderby", Some(q.sort));
	}
	if q.status.is_some() {
		qs.push_encoded("manga_status", q.status);
	}
	if q.genre.is_some() {
		qs.push_encoded("genre", q.genre);
	}
	if let Some(min) = q.ch_min {
		qs.push_encoded("ch_min", Some(&format!("{}", min as i64)));
	}
	if let Some(max) = q.ch_max {
		qs.push_encoded("ch_max", Some(&format!("{}", max as i64)));
	}
	if q.query.is_some() {
		qs.push("search", q.query);
	}

	let url = match (q.page > 1, qs.is_empty()) {
		(true, true) => format!("{BASE_URL}/manga/page/{}/", q.page),
		(true, false) => format!("{BASE_URL}/manga/page/{}/?{qs}", q.page),
		(false, true) => format!("{BASE_URL}/manga/"),
		(false, false) => format!("{BASE_URL}/manga/?{qs}"),
	};

	let html = Request::get(url)?.html()?;
	let entries = html
		.select("article.aqua-manga-card")
		.map(|els| els.filter_map(|el| parse_archive_card(&el)).collect())
		.unwrap_or_default();
	let has_next_page = html.select_first("a.next").is_some();

	Ok(MangaPageResult {
		entries,
		has_next_page,
	})
}

pub fn parse_archive_card(el: &Element) -> Option<Manga> {
	let link = el.select_first(".aqua-manga-card__title a")?;
	let key = link.attr("href")?.strip_prefix_or_self(BASE_URL).into();
	let title = link.text()?;
	if title.is_empty() {
		return None;
	}
	let cover = el
		.select_first("img.aqua-manga-card__cover")
		.and_then(|img| img.img_attr(false));
	Some(Manga {
		key,
		title,
		cover,
		..Default::default()
	})
}

pub fn content_rating(tags: &[String]) -> ContentRating {
	let mut suggestive = false;
	for tag in tags {
		if tag.eq_ignore_ascii_case("adult")
			|| tag.eq_ignore_ascii_case("erotica")
			|| tag.eq_ignore_ascii_case("mature")
			|| tag.eq_ignore_ascii_case("pornographic")
			|| tag.eq_ignore_ascii_case("r-18")
			|| tag.eq_ignore_ascii_case("sexual violence")
		{
			return ContentRating::NSFW;
		}
		if tag.eq_ignore_ascii_case("ecchi") || tag.eq_ignore_ascii_case("suggestive") {
			suggestive = true;
		}
	}
	if suggestive {
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

pub fn viewer(type_label: Option<&str>) -> Viewer {
	match type_label {
		Some(label) if label.eq_ignore_ascii_case("manga") => Viewer::RightToLeft,
		_ => Viewer::Webtoon,
	}
}

pub fn clean_chapter_title(raw: &str, chapter_number: Option<f32>) -> Option<String> {
	let lower = raw.to_ascii_lowercase();
	let rest = if let Some(s) = lower.strip_prefix("chapter") {
		&raw[raw.len() - s.len()..]
	} else if let Some(s) = lower.strip_prefix("ch.") {
		&raw[raw.len() - s.len()..]
	} else if let Some(s) = lower.strip_prefix("ch") {
		&raw[raw.len() - s.len()..]
	} else {
		return (!raw.is_empty()).then(|| String::from(raw));
	};

	let rest = rest.trim_start();
	if let Some(num) = chapter_number
		&& let Ok(parsed) = rest.parse::<f32>()
		&& (parsed - num).abs() < f32::EPSILON
	{
		return None;
	}

	let after_num = rest.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.');
	if let Some(sub) = after_num
		.trim_start()
		.strip_prefix([':', '-', '\u{2013}', '\u{2014}'])
	{
		let sub = sub.trim_start();
		if !sub.is_empty() {
			return Some(String::from(sub));
		}
	}

	(!raw.is_empty()).then(|| String::from(raw))
}

pub fn parse_chapter_date(raw: &str, params: &Params) -> Option<i64> {
	let text = raw.trim_start_matches(['\u{00b7}', ' ']);
	let split = text.find(|c: char| !c.is_ascii_digit())?;
	if split > 0
		&& let Ok(amount) = text[..split].parse::<i64>()
	{
		let unit = text[split..].trim_start();
		let unit_seconds = if unit.eq_ignore_ascii_case("s") {
			Some(1)
		} else if unit.eq_ignore_ascii_case("m") {
			Some(60)
		} else if unit.eq_ignore_ascii_case("h") {
			Some(3_600)
		} else if unit.eq_ignore_ascii_case("d") {
			Some(86_400)
		} else if unit.eq_ignore_ascii_case("w") {
			Some(604_800)
		} else if unit.eq_ignore_ascii_case("mo") {
			Some(2_592_000)
		} else if unit.eq_ignore_ascii_case("y") {
			Some(31_536_000)
		} else {
			None
		};
		if let Some(secs) = unit_seconds {
			return Some(current_date() - amount * secs);
		}
	}
	let ts = helpers::parse_chapter_date(params, text);
	(ts > 0).then_some(ts)
}

pub fn clean_excerpt(excerpt: &str, title: &str) -> Option<String> {
	let trimmed = excerpt
		.strip_prefix("Read ")
		.and_then(|rest| rest.strip_prefix(title))
		.map(str::trim_start)
		.unwrap_or(excerpt);
	(!trimmed.is_empty()).then(|| String::from(trimmed))
}

pub fn credit_list(html: &aidoku::imports::html::Document, selector: &str) -> Option<Vec<String>> {
	let mut out = Vec::new();
	for el in html.select(selector)? {
		if let Some(name) = el.text()
			&& !name.is_empty()
			&& !out.iter().any(|existing| existing == &name)
		{
			out.push(name);
		}
	}
	(!out.is_empty()).then_some(out)
}

pub fn listing(id: &str, name: &str) -> Listing {
	Listing {
		id: String::from(id),
		name: String::from(name),
		kind: ListingKind::Default,
	}
}

pub fn browse_filters() -> Vec<FilterItem> {
	let mut items = vec![
		FilterItem {
			title: String::from("Ongoing"),
			values: Some(vec![FilterValue::Select {
				id: String::from("manga_status"),
				value: String::from("on-going"),
			}]),
		},
		FilterItem {
			title: String::from("Completed"),
			values: Some(vec![FilterValue::Select {
				id: String::from("manga_status"),
				value: String::from("end"),
			}]),
		},
	];
	for &(title, slug) in BROWSE_GENRES {
		items.push(FilterItem {
			title: String::from(title),
			values: Some(vec![FilterValue::Select {
				id: String::from("genre"),
				value: String::from(slug),
			}]),
		});
	}
	items
}
