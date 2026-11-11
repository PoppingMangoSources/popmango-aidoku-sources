use crate::{Params, helpers};
use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Manga, Result,
	alloc::{String, Vec},
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};

pub fn fetch(params: &Params) -> Result<Document> {
	Ok(
		helpers::check_response(Request::get(&params.base_url)?.send()?, &params.base_url)?
			.get_html()?,
	)
}

fn manga(element: &Element, params: &Params) -> Option<Manga> {
	let child;
	let link = if element.attr("href").is_some() {
		element
	} else {
		child = element.select_first("a[href*='/series/']")?;
		&child
	};
	let href = link.attr("href")?;
	let path = href
		.strip_prefix(params.base_url.as_ref())
		.unwrap_or(&href)
		.strip_prefix("/series/")?;
	let slug = path
		.split(['/', '?', '#'])
		.next()
		.filter(|s| !s.trim().is_empty())?;
	let title = link
		.attr("title")
		.filter(|s| !s.trim().is_empty())
		.or_else(|| {
			element
				.select_first("h2, h3, p.display-6")
				.and_then(|e| e.text())
		})
		.or_else(|| element.select_first("img").and_then(|e| e.attr("alt")))?;
	if title.trim().is_empty() {
		return None;
	}
	Some(Manga {
		key: format!("/series/{slug}"),
		title: title.trim().into(),
		cover: element.select_first("img").and_then(|img| {
			img.attr("data-fallback-src")
				.filter(|s| s.starts_with("https://") && !s.contains("no-cover"))
				.or_else(|| img.attr("abs:src"))
		}),
		..Default::default()
	})
}

pub fn parse(html: &Document, params: &Params) -> Result<HomeLayout> {
	let mut components = Vec::new();
	let mut slides: Vec<_> = html
		.select(&params.home_slider_selector)
		.map(|slides| {
			slides
				.filter_map(|e| {
					let mut manga = manga(&e, params)?;
					manga.description = e
						.select_first("h3 + p, .text-lg, .em-hero-copy p.d-none")
						.and_then(|p| p.text())
						.map(|s| s.trim().into())
						.filter(|s: &String| !s.is_empty());
					let index = e
						.attr("data-swiper-slide-index")
						.and_then(|s| s.parse::<usize>().ok())
						.unwrap_or(0);
					Some((index, manga))
				})
				.collect()
		})
		.unwrap_or_default();
	// Looping carousels can rotate and duplicate their DOM slides.
	if !slides.is_sorted_by_key(|(index, _)| *index) {
		slides.sort_by_key(|(index, _)| *index);
	}
	let mut entries: Vec<Manga> = Vec::new();
	for (_, manga) in slides {
		if !entries.iter().any(|entry| entry.key == manga.key) {
			entries.push(manga);
		}
	}
	if !entries.is_empty() {
		components.push(HomeComponent {
			value: HomeComponentValue::BigScroller {
				entries,
				auto_scroll_interval: None,
			},
			..Default::default()
		});
	}
	if let Some(headings) = html.select("main h1, main h2, main h3") {
		for heading in headings {
			let Some(title) = heading
				.text()
				.map(|s| s.trim().into())
				.filter(|s: &String| params.home_section_titles.contains(&s.as_str()))
			else {
				continue;
			};
			if components
				.iter()
				.any(|c| c.title.as_deref() == Some(&title))
			{
				continue;
			}
			let mut parent = heading.parent();
			let mut entries: Vec<Link> = Vec::new();
			for _ in 0..3 {
				let Some(block) = parent else {
					break;
				};
				if block.tag_name().as_deref() == Some("main") {
					break;
				}
				let neighbors = block.select("h1, h2, h3").is_some_and(|hs| {
					hs.filter_map(|h| h.text()).any(|t| {
						t.trim() != title && params.home_section_titles.contains(&t.trim())
					})
				});
				if neighbors {
					break;
				}
				if let Some(cards) = block.select("a[href*='/series/']") {
					for card in cards {
						if card.select_first("img").is_none() {
							continue;
						}
						if let Some(manga) = manga(&card, params)
							&& !entries.iter().any(|entry| matches!(&entry.value, Some(LinkValue::Manga(existing)) if existing.key == manga.key)) { entries.push(Link::from(manga)); }
					}
				}
				if !entries.is_empty() {
					break;
				}
				parent = block.parent();
			}
			if entries.is_empty() {
				continue;
			}
			let ranking = matches!(title.as_str(), "Most Popular" | "Tendencias");
			components.push(HomeComponent {
				title: Some(title),
				value: if ranking {
					HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(5),
						entries,
						listing: None,
					}
				} else {
					HomeComponentValue::Scroller {
						entries,
						listing: None,
					}
				},
				..Default::default()
			});
		}
	}
	if components.is_empty() {
		bail!("Home sections not found");
	}
	Ok(HomeLayout { components })
}
