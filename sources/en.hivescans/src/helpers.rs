use crate::models::Banner;
use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, Result,
	alloc::{String, Vec},
	prelude::*,
};
use iken::{Params, home};

pub fn parse_home(html: &aidoku::imports::html::Document, params: &Params) -> Result<HomeLayout> {
	let mut layout = home::parse(html, params)?;
	// The remaining slides are initialized by the site's script.
	let data: String = html
		.select("script")
		.and_then(|scripts| {
			scripts
				.filter_map(|s| s.data().or_else(|| s.html()))
				.find_map(|s| {
					s.split_once("const hvtXHeroLibrary = ")
						.map(|(_, data)| data.into())
				})
		})
		.ok_or(error!("Banner data not found"))?;
	let banners = serde_json::Deserializer::from_str(&data)
		.into_iter::<Vec<Banner>>()
		.next()
		.ok_or(error!("Empty banner data"))??;
	let links: Vec<Link> = banners
		.into_iter()
		.take(4)
		.map(|banner| Manga::try_from(banner).map(Link::from))
		.collect::<Result<_>>()?;
	if links.is_empty() {
		bail!("No series banners");
	}
	layout.components.insert(
		0,
		HomeComponent {
			value: HomeComponentValue::ImageScroller {
				links,
				auto_scroll_interval: None,
				width: Some(400),
				height: Some(150),
			},
			..Default::default()
		},
	);
	Ok(layout)
}
