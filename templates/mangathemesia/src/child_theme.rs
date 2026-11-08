//! The home of a child theme whose rows `get_home` can't read: image banners, then `.bixbox`
//! boxes of `.bsx` cards in the site's order.
use crate::{
	Params,
	helpers::{ElementImageAttr, find_first_f32},
};
use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Manga, MangaStatus,
	MangaWithChapter, Result,
	alloc::Vec,
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};

pub fn get_home(params: &Params) -> Result<HomeLayout> {
	let base_url = &params.base_url;
	let html = Request::get(base_url.as_ref())?.html()?;
	let mut components: Vec<HomeComponent> = slider_banners(&html, params).into_iter().collect();

	// `.bixbox` rather than `div.bixbox`: Editor Picks is a <section>.
	for section in html.select(".bixbox").into_iter().flatten() {
		let Some(title) = section
			.select_first(".releases h2")
			.and_then(|h2| h2.text())
		else {
			continue;
		};
		let picks = editor_picks(&section, base_url);
		if !picks.is_empty() {
			components.push(HomeComponent {
				title: Some(title),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: picks,
					auto_scroll_interval: None,
				},
			});
			continue;
		}
		// Popular Today's box holds only its heading; its cards are in a rail beside it, inside
		// the `.hotslid` wrapper around both.
		let rail = section
			.parent()
			.filter(|parent| parent.has_class("hotslid"))
			.unwrap_or(section);
		let cards = rail.select(".bsx").into_iter().flatten();
		let value = if rail.select_first(".chapter-list > a").is_some() {
			let entries: Vec<MangaWithChapter> = cards
				.filter_map(|card| newest_chapter(&card, base_url))
				.collect();
			if entries.is_empty() {
				continue;
			}
			HomeComponentValue::MangaChapterList {
				page_size: None,
				entries,
				listing: None,
			}
		} else {
			let entries: Vec<Link> = cards
				.filter_map(|card| series(&card, base_url))
				.map(Link::from)
				.collect();
			if entries.is_empty() {
				continue;
			}
			HomeComponentValue::Scroller {
				entries,
				listing: None,
			}
		};
		components.push(HomeComponent {
			title: Some(title),
			subtitle: None,
			value,
		});
	}

	Ok(HomeLayout { components })
}

fn slider_banners(html: &Document, params: &Params) -> Option<HomeComponent> {
	let links: Vec<Link> = html
		.select(".slidernew .swiper-slide a")?
		.filter_map(|link| {
			let image = link.select_first("img")?;
			Some(Link {
				title: image.attr("alt")?,
				image_url: image.img_attr(),
				value: Some(LinkValue::Manga(Manga {
					key: link
						.attr("href")?
						.strip_prefix_or_self(&params.base_url)
						.into(),
					..Default::default()
				})),
				..Default::default()
			})
		})
		.collect();
	if links.is_empty() {
		return None;
	}
	Some(HomeComponent {
		title: None,
		subtitle: None,
		value: HomeComponentValue::ImageScroller {
			links,
			auto_scroll_interval: Some(5.0),
			width: params.banner_width,
			height: params.banner_height,
		},
	})
}

fn series(card: &Element, base_url: &str) -> Option<Manga> {
	let link = card.select_first("a")?;
	Some(Manga {
		key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: link.attr("title")?,
		cover: card.select_first("img").and_then(|image| image.img_attr()),
		..Default::default()
	})
}

// Coin-locked chapters need a login, and the chapter list drops them too.
fn newest_chapter(card: &Element, base_url: &str) -> Option<MangaWithChapter> {
	let link = card
		.select(".chapter-list > a")?
		.find(|link| link.select_first(".fa-coins").is_none())?;
	let title = link.select_first(".epxs")?.text()?;
	let chapter_number = find_first_f32(&title);
	Some(MangaWithChapter {
		manga: series(card, base_url)?,
		chapter: Chapter {
			key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
			title: chapter_number.is_none().then_some(title),
			chapter_number,
			..Default::default()
		},
	})
}

fn editor_picks(section: &Element, base_url: &str) -> Vec<Manga> {
	section
		.select(".violet-ep-panel")
		.into_iter()
		.flatten()
		.filter_map(|panel| {
			let link = panel.select_first(".violet-ep-series-title a")?;
			let index = panel.attr("data-index")?;
			Some(Manga {
				key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
				title: link.text()?,
				cover: section
					.select_first(format!(".violet-ep-cover[data-index=\"{index}\"] img"))
					.and_then(|image| image.img_attr()),
				description: panel
					.select_first(".violet-ep-tagline")
					.and_then(|tagline| tagline.text()),
				status: match panel
					.select_first(".violet-ep-badge--status")
					.and_then(|badge| badge.text())
					.map(|status| status.to_lowercase())
					.as_deref()
				{
					Some("ongoing") => MangaStatus::Ongoing,
					Some("completed") => MangaStatus::Completed,
					Some("hiatus") => MangaStatus::Hiatus,
					_ => MangaStatus::Unknown,
				},
				tags: panel
					.select(".violet-ep-genres li")
					.map(|genres| genres.filter_map(|genre| genre.text()).collect()),
				..Default::default()
			})
		})
		.collect()
}
