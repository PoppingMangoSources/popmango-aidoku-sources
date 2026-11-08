use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Manga, Result,
	alloc::{String, Vec},
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};
use mangathemesia::helpers::ElementImageAttr;

// The site's home is its own child theme, which the template's home doesn't read: a banner rail,
// rows of series cards and an editor's panel, in the site's order.
pub fn get_home(base_url: &str) -> Result<HomeLayout> {
	let html = Request::get(base_url)?.html()?;
	let mut components = Vec::new();

	let banners = banners(&html, base_url);
	if !banners.is_empty() {
		components.push(HomeComponent {
			title: None,
			subtitle: None,
			// The banners are 1983×793.
			value: HomeComponentValue::ImageScroller {
				links: banners,
				auto_scroll_interval: Some(5.0),
				width: Some(400),
				height: Some(160),
			},
		});
	}

	// Editor Picks is a <section>; the others are <div>s.
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
		let cards = section
			.parent()
			.filter(|parent| parent.has_class("hotslid"))
			.unwrap_or(section);
		let entries: Vec<Link> = cards
			.select(".bsx")
			.into_iter()
			.flatten()
			.filter_map(|card| series(&card, base_url))
			.map(Into::into)
			.collect();
		if !entries.is_empty() {
			components.push(HomeComponent {
				title: Some(title),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries,
					listing: None,
				},
			});
		}
	}

	Ok(HomeLayout { components })
}

fn banners(html: &Document, base_url: &str) -> Vec<Link> {
	html.select(".slidernew .swiper-slide a")
		.into_iter()
		.flatten()
		.filter_map(|link| {
			let image = link.select_first("img")?;
			Some(Link {
				title: image.attr("alt")?,
				image_url: image.img_attr(),
				value: Some(LinkValue::Manga(Manga {
					key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
					..Default::default()
				})),
				..Default::default()
			})
		})
		.collect()
}

// A card, unless it's a novel: novel chapters are text, which this source doesn't read.
fn series(card: &Element, base_url: &str) -> Option<Manga> {
	if card.select_first(".novelabel").is_some() {
		return None;
	}
	let link = card.select_first("a")?;
	Some(Manga {
		key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: link.attr("title")?,
		cover: card.select_first("img").and_then(|image| image.img_attr()),
		..Default::default()
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
				tags: panel.select(".violet-ep-genres li").map(|genres| {
					genres
						.filter_map(|genre| genre.text())
						.collect::<Vec<String>>()
				}),
				..Default::default()
			})
		})
		.collect()
}
