use crate::{
	OniSaga,
	api::{BASE_URL, SearchParams, html, parse},
	helpers::title_link,
	models::{BrowsePage, LatestUpdates, Spotlight, Work},
	rsc,
};
use aidoku::{
	AidokuError, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link,
	Listing, ListingKind, Manga, MangaWithChapter, Result,
	alloc::{String, Vec},
	imports::{html::Document, net::Request, std::send_partial_result},
	prelude::*,
};

// The site's home shelves show this many titles.
const SHELF_SIZE: i32 = 14;

pub const SECTIONS: [(&str, Option<&str>); 6] = [
	("Spotlight", None),
	("Trending this week", None),
	("Start a new series", Some("Recently added to OniSaga")),
	("Latest updates", None),
	("Finished series", None),
	("Newly published", Some("By original publication date")),
];

fn shelf(page: BrowsePage, id: &str, name: &str) -> HomeComponentValue {
	HomeComponentValue::Scroller {
		entries: page
			.items
			.into_iter()
			.filter_map(|work| Manga::try_from(work).ok().map(Link::from))
			.collect(),
		listing: Some(Listing {
			id: id.into(),
			name: name.into(),
			kind: ListingKind::Default,
		}),
	}
}

// Only some shelves label a title's origin.
fn links(doc: &Document, section: &str, origins: Option<&[String]>) -> Vec<Link> {
	doc.select(format!("section[aria-labelledby={section}] li"))
		.into_iter()
		.flatten()
		.filter(|item| {
			origins.is_none_or(|origins| {
				item.select_first("[data-medium]")
					.and_then(|badge| badge.attr("data-medium"))
					.is_none_or(|origin| origins.contains(&origin))
			})
		})
		.filter_map(|item| title_link(&item).map(Link::from))
		.collect()
}

fn keep<T>(result: Result<T>, failure: &mut Option<AidokuError>) -> Option<T> {
	result.map_err(|error| *failure = Some(error)).ok()
}

fn component((title, subtitle): &(&str, Option<&str>), value: HomeComponentValue) -> HomeComponent {
	HomeComponent {
		title: Some((*title).into()),
		subtitle: subtitle.map(Into::into),
		value,
	}
}

// A section whose response failed is left out.
pub fn components(
	search: &SearchParams,
	doc: Option<&Document>,
	new: Option<BrowsePage>,
	published: Option<BrowsePage>,
) -> Vec<HomeComponent> {
	// The home page ignores the reader's settings; search applies them on the server.
	let origins = search.origin.as_deref();
	let allowed = |work: &Work| work.allowed(&search.ratings, origins, &search.languages);
	let values = [
		doc.and_then(|doc| rsc::extract::<Spotlight>(doc, &["titles", "thumbs"]))
			.map(|spotlight| HomeComponentValue::BigScroller {
				entries: spotlight
					.thumbs
					.into_iter()
					.filter(allowed)
					.filter_map(|work| work.into_details().ok())
					.collect(),
				auto_scroll_interval: Some(8.0),
			}),
		doc.map(|doc| HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(3),
			entries: links(doc, "trending-title", origins),
			listing: None,
		}),
		new.map(|page| shelf(page, "new", "New on site")),
		doc.and_then(|doc| rsc::extract::<LatestUpdates>(doc, &["entries", "scope"]))
			.map(|latest| HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: latest
					.entries
					.into_iter()
					.filter(|entry| allowed(&entry.card))
					.filter_map(|entry| {
						let chapter = entry
							.units
							.into_iter()
							.find_map(|unit| unit.into_chapter(&search.languages))?;
						Some(MangaWithChapter {
							manga: Manga::try_from(entry.card).ok()?,
							chapter,
						})
					})
					.collect(),
				listing: None,
			}),
		doc.map(|doc| HomeComponentValue::Scroller {
			entries: links(doc, "finished-title", origins),
			listing: None,
		}),
		published.map(|page| shelf(page, "published", "First published")),
	];
	SECTIONS
		.iter()
		.zip(values)
		.filter_map(|(section, value)| Some(component(section, value?)))
		.collect()
}

impl Home for OniSaga {
	fn get_home(&self) -> Result<HomeLayout> {
		let search = SearchParams::new(None);
		if search.is_empty() {
			return Ok(HomeLayout::default());
		}
		if search.adult() {
			self.confirm_age()?;
		}
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: SECTIONS
				.iter()
				.zip([
					HomeComponentValue::empty_big_scroller(),
					HomeComponentValue::empty_manga_list(),
					HomeComponentValue::empty_scroller(),
					HomeComponentValue::empty_manga_chapter_list(),
					HomeComponentValue::empty_scroller(),
					HomeComponentValue::empty_scroller(),
				])
				.map(|(section, value)| component(section, value))
				.collect(),
		}));

		let shelf_request = |sort| {
			SearchParams {
				limit: SHELF_SIZE,
				..SearchParams::sorted(sort)
			}
			.request()
		};
		let mut responses = Request::send_all([
			Request::get(BASE_URL)?,
			shelf_request("new")?,
			shelf_request("published")?,
		])
		.into_iter();
		let mut failure = None;
		let mut next = || {
			responses
				.next()
				.ok_or_else(|| error!("OniSaga didn't answer"))
		};
		let doc = keep(next().and_then(|r| html(r?)), &mut failure);
		let new = keep(next().and_then(|r| parse::<BrowsePage>(r?)), &mut failure);
		let published = keep(next().and_then(|r| parse::<BrowsePage>(r?)), &mut failure);
		let components = components(&search, doc.as_ref(), new, published);
		if components.is_empty() {
			return Err(failure.unwrap_or_else(|| error!("OniSaga's home page was empty")));
		}
		Ok(HomeLayout { components })
	}
}
