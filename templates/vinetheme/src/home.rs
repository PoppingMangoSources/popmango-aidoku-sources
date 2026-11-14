use crate::{
	Params,
	helpers::{absolute_url, link_keys, plain_text, resolve_urls},
	models::{Series, compact},
	rsc, settings,
};
use aidoku::{
	Chapter, ContentRating, HomeComponent, HomeComponentValue, HomeLayout, Link, Manga,
	MangaWithChapter, Result,
	alloc::{
		String, Vec,
		collections::{BTreeMap, BTreeSet, btree_map::Entry},
	},
	imports::{
		html::{Element, Html},
		std::current_date,
	},
	prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Slides {
	initial_slides: Vec<Series>,
}

pub fn parse(body: &str, params: &Params) -> Result<HomeLayout> {
	let html = Html::parse(body)?;
	let flight = rsc::decode(body)?;
	let mut metadata = BTreeMap::new();
	for value in rsc::objects(&flight, &["slug", "title", "coverImage"]) {
		if let Ok(mut series) = serde_json::from_value::<Series>(value) {
			let chapters = core::mem::take(&mut series.chapters);
			let description = series.description.take().and_then(|text| plain_text(&text));
			if let Ok(mut manga) = Manga::try_from(series) {
				manga.description = description;
				resolve_urls(&mut manga, &params.base_url);
				let chapters: Vec<_> = chapters
					.into_iter()
					.map(|c| c.into_chapter(params, &manga.key))
					.collect();
				match metadata.entry(manga.key.clone()) {
					Entry::Vacant(entry) => {
						entry.insert((manga, chapters));
					}
					Entry::Occupied(mut entry) => {
						let (old, old_chapters) = entry.get_mut();
						if old.description.is_none() {
							old.description = manga.description;
						}
						if manga.content_rating == ContentRating::NSFW {
							old.content_rating = ContentRating::NSFW;
						}
						if old.authors.is_none() {
							old.authors = manga.authors;
						}
						if old.artists.is_none() {
							old.artists = manga.artists;
						}
						if old.tags.as_ref().is_none_or(Vec::is_empty) {
							old.tags = manga.tags;
						}
						if old_chapters.is_empty() {
							*old_chapters = chapters;
						}
					}
				}
			}
		}
	}
	let mut components = Vec::new();
	if let Ok(slides) = rsc::extract::<Slides>(&flight, &["initialSlides"]) {
		let entries: Vec<_> = slides
			.initial_slides
			.into_iter()
			.filter_map(|mut series| {
				let description = series.description.take().and_then(|text| plain_text(&text));
				let mut manga = Manga::try_from(series).ok()?;
				resolve_urls(&mut manga, &params.base_url);
				manga.description = description;
				(settings::show_adult() || manga.content_rating != ContentRating::NSFW)
					.then_some(manga)
			})
			.collect();
		if !entries.is_empty() {
			components.push(HomeComponent {
				title: None,
				value: HomeComponentValue::BigScroller {
					entries,
					auto_scroll_interval: Some(8.0),
				},
				..Default::default()
			});
		}
	}
	let mut titles = BTreeSet::new();
	for section in html
		.select("section, [data-featured-band], [data-new-series-section]")
		.ok_or(error!("Missing home sections"))?
	{
		// A featured stack has an outer section around the actual row.
		if section.children().any(|child| {
			child.tag_name().as_deref() == Some("section")
				|| child.select_first("section").is_some()
		}) {
			continue;
		}
		let title = if section.attr("data-featured-band").is_some()
			|| section.select_first("[data-featured-band]").is_some()
		{
			Some("Featured".into())
		} else {
			section.select_first("h2").and_then(|h| h.text())
		};
		let Some(title) = title else { continue };
		if title == "On Sale Now" || !titles.insert(title.clone()) {
			continue;
		}
		let Some(links) = section.select("a[href*='ref=homepage_']") else {
			continue;
		};
		let mut seen = BTreeSet::new();
		let mut manga = Vec::new();
		let mut updates = Vec::new();
		let mut featured = false;
		let mut ranking = false;
		let listing = section
			.select_first("a[href^='/series?']")
			.and_then(|link| link.attr("href"))
			.and_then(|href| {
				href.split_once('?').map(|(_, query)| aidoku::Listing {
					id: query.into(),
					name: title.clone(),
					..Default::default()
				})
			});
		for link in links {
			let Some(href) = link.attr("href") else {
				continue;
			};
			let Some((key, None)) = link_keys(&href) else {
				continue;
			};
			if !seen.insert(key.clone()) {
				continue;
			}
			featured |= href.contains("ref=homepage_featured");
			ranking |= href.contains("ref=homepage_most_popular");
			let Some(entry) = metadata
				.get(&key)
				.map(|(manga, _)| manga.clone())
				.or_else(|| from_link(&link, &key, params))
			else {
				continue;
			};
			if !settings::show_adult() && entry.content_rating == ContentRating::NSFW {
				continue;
			}
			if href.contains("ref=homepage_latest") {
				let chapter = match metadata.get(&key) {
					Some((_, chapters)) if !chapters.is_empty() => chapters
						.iter()
						.find(|c| settings::show_locked() || !c.locked)
						.cloned(),
					_ => from_card(&link, &key, params),
				};
				if let Some(chapter) = chapter {
					updates.push(MangaWithChapter {
						manga: compact(entry),
						chapter,
					});
				}
			} else {
				manga.push(entry);
			}
		}
		let value = if !updates.is_empty() {
			HomeComponentValue::MangaChapterList {
				entries: updates,
				page_size: Some(5),
				listing,
			}
		} else if !manga.is_empty() {
			if featured {
				HomeComponentValue::BigScroller {
					entries: manga,
					auto_scroll_interval: Some(8.0),
				}
			} else if ranking {
				HomeComponentValue::MangaList {
					entries: manga.into_iter().map(compact).map(Link::from).collect(),
					ranking: true,
					page_size: Some(5),
					listing,
				}
			} else {
				HomeComponentValue::Scroller {
					entries: manga.into_iter().map(compact).map(Link::from).collect(),
					listing,
				}
			}
		} else {
			continue;
		};
		components.push(HomeComponent {
			title: Some(title),
			value,
			..Default::default()
		});
	}
	if components.is_empty() {
		bail!("No home content found");
	}
	Ok(HomeLayout { components })
}

fn from_link(link: &Element, key: &str, params: &Params) -> Option<Manga> {
	let card = link.parent()?;
	let title = link
		.select_first("h2, h3")
		.and_then(|h| h.text())
		.or_else(|| link.select_first("img").and_then(|i| i.attr("alt")))
		.or_else(|| card.select_first("h2, h3").and_then(|h| h.text()))?;
	if title.trim().is_empty() {
		return None;
	}
	let cover = link
		.select_first("img")
		.or_else(|| card.select_first("img"))
		.and_then(|i| i.attr("src"))
		.map(|path| absolute_url(&params.base_url, &path));
	Some(Manga {
		key: key.into(),
		title: if key.starts_with("novel/") && settings::label_novels() {
			format!("{title} (Novel)")
		} else {
			title
		},
		cover,
		..Default::default()
	})
}

fn from_card(link: &Element, manga_key: &str, params: &Params) -> Option<Chapter> {
	let mut card = link.parent()?;
	for _ in 0..3 {
		if let Some(chapters) = card.select("a[href*='/chapter/']") {
			for link in chapters {
				let locked = link
					.select_first("[data-art='M-ch-locked'], [data-art='M-coin']")
					.is_some();
				if locked && !settings::show_locked() {
					continue;
				}
				let href = link.attr("href")?;
				let (series, Some(key)) = link_keys(&href)? else {
					continue;
				};
				if series != manga_key {
					continue;
				}
				let date_uploaded = link
					.select_first("[data-chapter-row-time]")
					.and_then(|t| t.text())
					.and_then(|t| relative_date(&t));
				return Some(Chapter {
					chapter_number: key.parse().ok(),
					key,
					locked,
					date_uploaded,
					url: Some(absolute_url(&params.base_url, &href)),
					..Default::default()
				});
			}
		}
		card = card.parent()?;
	}
	None
}

fn relative_date(value: &str) -> Option<i64> {
	let digits: String = value.chars().take_while(char::is_ascii_digit).collect();
	let value = value.trim();
	let amount: i64 = digits.parse().ok()?;
	let scale = match value.get(digits.len()..)? {
		"m" => 60,
		"h" => 3600,
		"d" => 86400,
		"w" => 604800,
		_ => return None,
	};
	Some(current_date() as i64 - amount * scale)
}
