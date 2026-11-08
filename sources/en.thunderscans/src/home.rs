use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, MangaWithChapter, Result,
	alloc::Vec,
	helpers::string::StripPrefixOrSelf,
	imports::{html::Element, net::Request},
};
use mangathemesia::{
	Params,
	helpers::{ElementImageAttr, find_first_f32, slider_banners},
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
		// Latest Comics and Latest Novels list each series' newest chapters.
		if section.select_first(".chapter-list > a").is_some() {
			let entries: Vec<MangaWithChapter> = section
				.select(".bsx")
				.into_iter()
				.flatten()
				.filter_map(|card| newest_chapter(&card, base_url))
				.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some(title),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: None,
						entries,
						listing: None,
					},
				});
			}
			continue;
		}
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

// The newest chapter a reader can open: coin-locked ones need a login, and the template drops them
// from chapter lists too.
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
