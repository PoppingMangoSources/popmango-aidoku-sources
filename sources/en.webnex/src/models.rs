use crate::{
	BASE_URL,
	helpers::{date, markdown, parse_flight_value, parse_key, parse_relative_date, text},
	settings::Groups,
};
use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Page, PageContent, Result, Viewer,
	alloc::{
		String, Vec,
		collections::{BTreeMap, BTreeSet},
		vec,
	},
	imports::html::{Document, Element},
	prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Taxonomy {
	genres: Vec<String>,
	tags: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderProps {
	pub series: SeriesRef,
	pub pages_url: String,
}

#[derive(Deserialize)]
pub struct SeriesRef {
	pub href: String,
}

#[derive(Deserialize)]
pub struct ReaderPages {
	#[serde(default)]
	pub pages: Vec<String>,
	pub message: Option<String>,
}

impl TryFrom<ReaderPages> for Vec<Page> {
	type Error = aidoku::AidokuError;

	fn try_from(data: ReaderPages) -> Result<Self> {
		if data.pages.is_empty() {
			bail!(
				"{}",
				data.message
					.as_deref()
					.unwrap_or("WebNex returned no pages for this chapter")
			);
		}
		data.pages
			.into_iter()
			.map(|url| {
				let url = if url.starts_with('/') && !url.starts_with("//") {
					format!("{BASE_URL}{url}")
				} else if url.starts_with("https://") {
					url
				} else {
					bail!("WebNex returned an invalid page URL");
				};
				Ok(Page {
					content: PageContent::url(url),
					..Default::default()
				})
			})
			.collect()
	}
}

pub fn details(root: &Document, key: &str) -> Result<Manga> {
	let texts = |selector| {
		root.select(selector)
			.into_iter()
			.flatten()
			.filter_map(|e| e.text())
			.filter(|s| !s.is_empty())
			.collect::<Vec<_>>()
	};
	let credits = |selector| {
		let mut seen = BTreeSet::new();
		let names: Vec<_> = texts(selector)
			.into_iter()
			.filter(|name| seen.insert(name.clone()))
			.collect();
		Some(names).filter(|names| !names.is_empty())
	};
	let title = root
		.select_first("h1.series-title")
		.and_then(|e| e.text())
		.filter(|s| !s.is_empty())
		.ok_or_else(|| error!("WebNex returned no series title"))?;
	let mut tags = texts(".series-badges a[href*=kind], .series-badges a[href*=demographic]");
	if let Some(taxonomy) = parse_flight_value::<Taxonomy>(root, "{\"genres\":[") {
		tags.extend(taxonomy.genres);
		tags.extend(taxonomy.tags);
	} else {
		tags.extend(texts(".detail-list a.chip"));
	}
	tags.sort_unstable();
	tags.dedup();
	let facts = texts(".series-badges > li");
	// Safe titles have no rating badge.
	let mut rating = if facts.is_empty() {
		ContentRating::Unknown
	} else {
		ContentRating::Safe
	};
	let mut viewer = Viewer::Unknown;
	let mut status = MangaStatus::Unknown;
	for fact in facts {
		match fact.to_ascii_lowercase().as_str() {
			"ongoing" => status = MangaStatus::Ongoing,
			"completed" => status = MangaStatus::Completed,
			"hiatus" | "on hiatus" => status = MangaStatus::Hiatus,
			"dropped" | "cancelled" => status = MangaStatus::Cancelled,
			"manga" => viewer = Viewer::RightToLeft,
			"manhwa" | "manhua" => viewer = Viewer::Webtoon,
			"safe" => rating = ContentRating::Safe,
			"suggestive" => rating = ContentRating::Suggestive,
			"18+" | "erotica" | "pornographic" => rating = ContentRating::NSFW,
			_ => {}
		}
	}
	Ok(Manga {
		key: key.into(),
		title,
		cover: root
			.select_first(".series-top-cover img")
			.and_then(|e| e.attr("abs:src")),
		authors: credits(".series-credits a[href*=author]"),
		artists: credits(".series-credits a[href*=artist]"),
		description: Some(
			root.select(".series-desc p")
				.into_iter()
				.flatten()
				.map(|paragraph| {
					let mut text = String::new();
					markdown(&paragraph, &mut text);
					text.trim().into()
				})
				.filter(|text: &String| !text.is_empty())
				.collect::<Vec<_>>()
				.join("\n\n"),
		)
		.filter(|text| !text.is_empty()),
		tags: Some(tags),
		viewer,
		status,
		content_rating: rating,
		url: Some(format!("{BASE_URL}/manga/{key}")),
		..Default::default()
	})
}

pub struct Upload {
	chapter: Chapter,
	// Lowercased, to match the group settings.
	group: Option<String>,
	warning: bool,
	likes: u32,
}

impl Upload {
	fn priority(&self, groups: &Groups, other: &Self) -> core::cmp::Ordering {
		let rank = |u: &Self| {
			(
				groups
					.preferred
					.iter()
					.position(|g| u.group.as_ref() == Some(g))
					.unwrap_or(usize::MAX),
				u.warning,
				core::cmp::Reverse(u.likes),
			)
		};
		rank(self)
			.cmp(&rank(other))
			.then_with(|| self.chapter.key.cmp(&other.chapter.key))
	}
}

pub fn uploads(doc: &Document, groups: &Groups, now: i64) -> Result<Vec<Upload>> {
	let mut result = Vec::new();
	for row in doc
		.select("li.ch-row:not([aria-hidden=true]):not([style*='display:none'])")
		.into_iter()
		.flatten()
	{
		let number = row
			.select_first(".ch-num")
			.and_then(|el| el.text())
			.and_then(|s| {
				s.trim_start_matches(|c: char| !c.is_ascii_digit())
					.trim()
					.parse()
					.ok()
			});
		let title = text(&row, ".ch-title");
		let mut uploads = Vec::new();
		let mut upload = |link: Element,
		                  group: Option<String>,
		                  when,
		                  official,
		                  warning,
		                  likes: Option<String>|
		 -> Result<()> {
			let Some(href) = link.attr("href") else {
				return Ok(());
			};
			let key = parse_key(&href, "/read/")
				.ok_or_else(|| error!("WebNex returned an invalid chapter link"))?;
			let name = group.as_deref().map(str::to_lowercase);
			if name
				.as_ref()
				.is_some_and(|name| groups.hidden.contains(name))
			{
				return Ok(());
			}
			uploads.push(Upload {
				chapter: Chapter {
					key: key.into(),
					title: title.clone(),
					chapter_number: number,
					date_uploaded: when,
					scanlators: group.map(|name| {
						vec![if official {
							format!("{name} ✓")
						} else {
							name
						}]
					}),
					url: Some(format!("{BASE_URL}/read/{key}")),
					..Default::default()
				},
				group: name,
				warning,
				likes: likes
					.and_then(|s| s.split_whitespace().next()?.parse().ok())
					.unwrap_or_default(),
			});
			Ok(())
		};
		if let Some(link) = row.select_first("a.ch-link") {
			upload(
				link,
				text(&row, ".ch-source-name, .ch-credit-group")
					.or_else(|| text(&row, ".ch-source")),
				row.select_first("time.ch-time").and_then(|e| date(&e)),
				row.select_first(".ch-source .verified-mark, .ch-source-trigger .verified-mark, .ch-sources-item.is-current .verified-mark").is_some(),
				false,
				text(&row, ".ch-likes"),
			)?;
		}
		for link in row.select("a.ch-sources-item").into_iter().flatten() {
			let name = text(&link, ".ch-sources-name");
			let time = text(&link, ".ch-sources-time").and_then(|v| parse_relative_date(&v, now));
			let warning = link.select_first(".dot-warn").is_some();
			let official = link.select_first(".verified-mark").is_some();
			let likes = text(&link, ".ch-sources-likes");
			upload(link, name, time, official, warning, likes)?;
		}
		// priority breaks every tie by key, so the unstable sort gives one order.
		uploads.sort_unstable_by(|a, b| a.priority(groups, b));
		let mut seen = Vec::new();
		for u in uploads {
			if let Some(group) = &u.group {
				if number.is_some() && seen.contains(group) {
					continue;
				}
				seen.push(group.clone());
			}
			result.push(u);
		}
	}
	Ok(result)
}

pub fn select_chapters(uploads: Vec<Upload>, groups: &Groups) -> Vec<Chapter> {
	let mut winners = BTreeMap::new();
	if groups.one_per_chapter {
		for (i, u) in uploads.iter().enumerate() {
			if let Some(number) = u.chapter.chapter_number {
				let best = winners.entry(number.to_bits()).or_insert(i);
				if u.priority(groups, &uploads[*best]).is_lt() {
					*best = i;
				}
			}
		}
	}
	let winners: BTreeMap<_, _> = winners
		.into_iter()
		.map(|(n, i)| (n, (i, uploads[i].group.clone())))
		.collect();
	uploads
		.into_iter()
		.enumerate()
		.filter_map(|(i, u)| {
			if let Some((winner, group)) = u
				.chapter
				.chapter_number
				.and_then(|n| winners.get(&n.to_bits()))
				&& i != *winner
				&& (u.group.is_none() || group != &u.group)
			{
				return None;
			}
			Some(u.chapter)
		})
		.collect()
}
