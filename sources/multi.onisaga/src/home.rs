use crate::{
	OniSaga,
	api::{SearchParams, parse, rpc},
	models::{BrowsePage, Empty, HomeData, Work},
};
use aidoku::{
	AidokuError, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link,
	Listing, ListingKind, Manga, MangaWithChapter, Result,
	alloc::Vec,
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

pub const SECTIONS: [(&str, Option<&str>); 6] = [
	("Spotlight", None),
	("Trending this week", None),
	("Start a new series", Some("Recently added to OniSaga")),
	("Latest updates", None),
	("Finished series", None),
	("Newly published", Some("By original publication date")),
];

fn shelf(
	items: Vec<Work>,
	filter_by: Option<&SearchParams>,
	id: &str,
	name: &str,
) -> Option<HomeComponentValue> {
	let entries = items
		.into_iter()
		.filter(|work| {
			filter_by.is_none_or(|search| {
				work.allowed(&search.ratings, search.origin.as_deref(), &search.languages)
			})
		})
		.filter_map(|work| Manga::try_from(work).ok().map(Link::from))
		.collect::<Vec<_>>();
	(!entries.is_empty()).then(|| HomeComponentValue::Scroller {
		entries,
		listing: Some(Listing {
			id: id.into(),
			name: name.into(),
			kind: ListingKind::Default,
		}),
	})
}

fn keep<T>(result: Result<T>, failure: &mut Option<AidokuError>) -> Option<T> {
	result.map_err(|err| *failure = Some(err)).ok()
}

fn component((title, subtitle): &(&str, Option<&str>), value: HomeComponentValue) -> HomeComponent {
	HomeComponent {
		title: Some((*title).into()),
		subtitle: subtitle.map(Into::into),
		value,
	}
}

pub fn components(
	search: &SearchParams,
	home: Option<HomeData>,
	new: Option<BrowsePage>,
	finished: Option<BrowsePage>,
	published: Option<BrowsePage>,
) -> Vec<HomeComponent> {
	let origins = search.origin.as_deref();
	let (spotlight, trending, home_new, latest, home_finished, home_published) = match home {
		Some(data) => (
			data.spotlight.value,
			Some(data.trending),
			data.new_on_site.value,
			data.latest.value,
			data.finished.value,
			data.newly_published.value,
		),
		None => (None, None, None, None, None, None),
	};

	let spotlight_value = spotlight.and_then(|slides| {
		let entries = slides
			.into_iter()
			.filter_map(|slide| slide.card)
			.filter(|work| work.allowed(&search.ratings, origins, &search.languages))
			.filter_map(|work| work.into_details().ok())
			.collect::<Vec<_>>();
		(!entries.is_empty()).then_some(HomeComponentValue::BigScroller {
			entries,
			auto_scroll_interval: Some(8.0),
		})
	});

	let trending_value = trending.and_then(|items| {
		let entries = items
			.into_iter()
			.filter(|item| item.allowed(&search.ratings, origins))
			.filter_map(|item| Manga::try_from(item).ok().map(Link::from))
			.collect::<Vec<_>>();
		(!entries.is_empty()).then_some(HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(3),
			entries,
			listing: None,
		})
	});

	let new_value = match new {
		Some(page) => shelf(page.items, None, "new", "New on site"),
		None => home_new.and_then(|items| shelf(items, Some(search), "new", "New on site")),
	};

	let latest_value = latest.and_then(|entries| {
		let entries = entries
			.into_iter()
			.filter_map(|entry| {
				let card = entry.card?;
				if !card.allowed(&search.ratings, origins, &search.languages) {
					return None;
				}
				let chapter = entry
					.units
					.into_iter()
					.find_map(|unit| unit.into_chapter(&search.languages))?;
				Some(MangaWithChapter {
					manga: Manga::try_from(card).ok()?,
					chapter,
				})
			})
			.collect::<Vec<_>>();
		(!entries.is_empty()).then(|| HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries,
			listing: Some(Listing {
				id: "updated".into(),
				name: "Latest update".into(),
				kind: ListingKind::Default,
			}),
		})
	});

	let finished_value = match finished {
		Some(page) => shelf(page.items, None, "finished", "Finished series"),
		None => home_finished
			.and_then(|items| shelf(items, Some(search), "finished", "Finished series")),
	};

	let published_value = match published {
		Some(page) => shelf(page.items, None, "published", "First published"),
		None => home_published
			.and_then(|items| shelf(items, Some(search), "published", "First published")),
	};

	SECTIONS
		.iter()
		.zip([
			spotlight_value,
			trending_value,
			new_value,
			latest_value,
			finished_value,
			published_value,
		])
		.filter_map(|(section, value)| Some(component(section, value?)))
		.collect()
}

impl Home for OniSaga {
	fn get_home(&self) -> Result<HomeLayout> {
		let search = SearchParams::new(None);
		if search.is_empty() {
			return Ok(HomeLayout::default());
		}
		self.sync_age(search.adult())?;

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

		let mut failure = None;
		let (home, new, finished, published) = if search.shelves_narrowed() {
			let new_req = SearchParams {
				limit: 18,
				..SearchParams::sorted("new")
			}
			.request()?;
			let finished_req = SearchParams {
				limit: 18,
				status: Vec::from(["completed".into()]),
				..SearchParams::sorted("updated")
			}
			.request()?;
			let published_req = SearchParams {
				limit: 18,
				..SearchParams::sorted("published")
			}
			.request()?;
			let mut responses = Request::send_all([
				rpc("catalog/home", Empty {})?,
				new_req,
				finished_req,
				published_req,
			])
			.into_iter();
			let mut next = || {
				responses
					.next()
					.ok_or_else(|| error!("OniSaga didn't answer"))
			};
			(
				keep(next().and_then(|res| parse::<HomeData>(res?)), &mut failure),
				keep(
					next().and_then(|res| parse::<BrowsePage>(res?)),
					&mut failure,
				),
				keep(
					next().and_then(|res| parse::<BrowsePage>(res?)),
					&mut failure,
				),
				keep(
					next().and_then(|res| parse::<BrowsePage>(res?)),
					&mut failure,
				),
			)
		} else {
			let home = keep(
				rpc("catalog/home", Empty {})?
					.send()
					.map_err(Into::into)
					.and_then(parse),
				&mut failure,
			);
			(home, None, None, None)
		};

		let components = components(&search, home, new, finished, published);
		if components.is_empty() {
			return Err(failure.unwrap_or_else(|| error!("OniSaga's home page was empty")));
		}
		Ok(HomeLayout { components })
	}
}
