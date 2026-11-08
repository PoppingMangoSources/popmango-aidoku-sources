use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, Result,
	alloc::Vec,
	helpers::string::StripPrefixOrSelf,
	imports::{html::Element, net::Request},
};
use mangathemesia::{
	Params,
	helpers::{ElementImageAttr, slider_banners},
};

// The site's home is its own child theme, which the template's home doesn't read beyond its banners:
// rows of series cards, in the site's order.
pub fn get_home(params: &Params) -> Result<HomeLayout> {
	let base_url = &params.base_url;
	let html = Request::get(base_url.as_ref())?.html()?;
	let mut components = Vec::new();

	components.extend(slider_banners(&html, params));

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

fn series(card: &Element, base_url: &str) -> Option<Manga> {
	let link = card.select_first("a")?;
	Some(Manga {
		key: link.attr("href")?.strip_prefix_or_self(base_url).into(),
		title: link.attr("title")?,
		cover: card.select_first("img").and_then(|image| image.img_attr()),
		..Default::default()
	})
}
