use crate::{
	BASE_URL, WebNex,
	helpers::{browse, card, get_html, parse_key, parse_relative_date, text},
	settings::{CONTENT_RATINGS, Catalogue},
};
use aidoku::{
	Chapter, Home, HomeComponent, HomeComponentValue, HomeLayout, Link, MangaWithChapter, Result,
	alloc::{Vec, string::ToString, vec},
	imports::{
		html::{Document, Element},
		std::current_date,
	},
	prelude::*,
};

// Streamed cards live in the matching S: chunk, in placeholder order.
fn cards(doc: &Document, row: &Element) -> Vec<Element> {
	let mut cards = Vec::new();
	for el in row
		.select("li.card-item, .side-list > li, template[id^='P:']")
		.into_iter()
		.flatten()
	{
		if let Some(id) = el
			.attr("id")
			.and_then(|s| s.strip_prefix("P:").map(|s| s.to_string()))
		{
			if let Some(chunk) = doc.select_first(format!("div[id='S:{id}']")) {
				cards.extend(self::cards(doc, &chunk));
			}
		} else {
			cards.push(el);
		}
	}
	cards
}

pub fn layout(doc: &Document, now: i64) -> HomeLayout {
	let mut components = Vec::new();
	for (id, selector) in [
		("popular", "section[aria-labelledby=popular-heading]"),
		("taste", "section[aria-labelledby=taste-heading]"),
		("new", "section[aria-labelledby=new-heading]"),
		("latest", "section[aria-labelledby=latest-heading]"),
		("trending", "section[aria-labelledby=trending-heading]"),
		("finished", "[role=tabpanel][id$=-finished]"),
		("added", "[role=tabpanel][id$=-added]"),
	] {
		let Some(section) = doc.select_first(selector) else {
			continue;
		};
		let elements = cards(doc, &section);
		let title = match id {
			"popular" => Some("Popular This Week".into()),
			"finished" => Some("Finished".into()),
			"added" => Some("Just Added".into()),
			_ => text(&section, "h2"),
		};
		let value = match id {
			"popular" => {
				let entries: Vec<_> = elements
					.iter()
					.filter_map(|el| {
						let mut manga = card(el)?;
						let readers = text(el, ".card-meta span:nth-child(2)");
						let rank = text(el, ".card-rank");
						manga.description = match (rank, readers) {
							(Some(rank), Some(readers)) => Some(format!("#{rank} · {readers}")),
							(Some(rank), None) => Some(format!("#{rank}")),
							(None, readers) => readers,
						};
						manga.tags = text(el, ".card-kind, .card-meta span:first-child")
							.filter(|s| matches!(s.as_str(), "Manga" | "Manhwa" | "Manhua"))
							.map(|s| vec![s]);
						Some(manga)
					})
					.collect();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::BigScroller {
					entries,
					auto_scroll_interval: Some(5.0),
				}
			}
			"latest" => {
				let entries: Vec<_> = elements
					.iter()
					.filter_map(|el| {
						let link = el.select_first(".card-chapters a[href^='/read/']")?;
						let key = parse_key(&link.attr("href")?, "/read/")?.to_string();
						let label = text(&link, ".card-ch-label")?;
						let n = label.trim_start_matches("Ch.").trim().parse().ok();
						Some(MangaWithChapter {
							manga: card(el)?,
							chapter: Chapter {
								url: Some(format!("{BASE_URL}/read/{key}")),
								key,
								chapter_number: n,
								title: n.is_none().then_some(label),
								date_uploaded: text(&link, ".card-ch-time")
									.and_then(|t| parse_relative_date(&t, now)),
								..Default::default()
							},
						})
					})
					.collect();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::MangaChapterList {
					entries,
					page_size: Some(4),
					listing: None,
				}
			}
			_ => {
				let entries: Vec<_> = elements.iter().filter_map(card).map(Link::from).collect();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::Scroller {
					entries,
					listing: None,
				}
			}
		};
		components.push(HomeComponent {
			title,
			subtitle: None,
			value,
		});
	}
	HomeLayout { components }
}

impl Home for WebNex {
	fn get_home(&self) -> Result<HomeLayout> {
		let prefs = Catalogue::load();
		let Some(max) = prefs.ceiling().filter(|_| !prefs.types.is_empty()) else {
			return Ok(HomeLayout::default());
		};
		// Home supports a rating ceiling; browse also supports individual ratings.
		if CONTENT_RATINGS
			.iter()
			.take_while(|s| **s != max)
			.any(|r| !prefs.ratings.iter().any(|v| v == r))
		{
			let result = browse(None, 1, vec![], prefs)?;
			return Ok(HomeLayout {
				components: vec![HomeComponent {
					title: Some("Filtered Catalogue".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: result.entries.into_iter().map(Link::from).collect(),
						listing: None,
					},
				}],
			});
		}
		let doc = get_html(BASE_URL, Some(&prefs.cookie(max)))?;
		let home = layout(&doc, current_date());
		if home.components.is_empty() && doc.select_first(".home-section").is_none() {
			bail!("WebNex did not return the home page");
		}
		Ok(home)
	}
}
