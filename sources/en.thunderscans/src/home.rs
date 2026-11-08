use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Manga, Result,
	alloc::Vec,
	helpers::string::StripPrefixOrSelf,
	imports::{
		html::{Document, Element},
		net::Request,
	},
};
use mangathemesia::helpers::ElementImageAttr;

// The site's home is its own child theme, which the template's home doesn't read: a banner rail,
// rows of series cards, in the site's order.
pub fn get_home(base_url: &str) -> Result<HomeLayout> {
	let html = Request::get(base_url)?.html()?;
	let mut components = Vec::new();

	let banners = banners(&html, base_url);
	if !banners.is_empty() {
		components.push(HomeComponent {
			title: None,
			subtitle: None,
			// The banners are 2048×768.
			value: HomeComponentValue::ImageScroller {
				links: banners,
				auto_scroll_interval: Some(5.0),
				width: Some(400),
				height: Some(150),
			},
		});
	}

	for section in html.select("div.bixbox").into_iter().flatten() {
		let Some(title) = section
			.select_first(".releases h2")
			.and_then(|h2| h2.text())
		else {
			continue;
		};
		let entries: Vec<Link> = section
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
