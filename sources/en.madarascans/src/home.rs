use aidoku::{
	Chapter, HomeComponent, HomeComponentValue, HomeLayout, Link, Manga, MangaStatus,
	MangaWithChapter, Result,
	alloc::{String, Vec},
	helpers::string::StripPrefixOrSelf,
	imports::{html::Element, net::Request},
	prelude::*,
};
use mangathemesia::{
	Params,
	helpers::{ElementImageAttr, find_first_f32, slider_banners},
};

// The site's home is its own child theme, which the template's home doesn't read beyond its banners:
// rows of series cards and an editor's panel, in the site's order.
pub fn get_home(params: &Params) -> Result<HomeLayout> {
	let base_url = &params.base_url;
	let html = Request::get(base_url.as_ref())?.html()?;
	let mut components = Vec::new();

	components.extend(slider_banners(&html, params));

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
		// Latest Comics and Latest Novels list each series' newest chapters.
		if cards.select_first(".chapter-list > a").is_some() {
			let entries: Vec<MangaWithChapter> = cards
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
