use crate::{
	Swarm,
	api::{API_URL, Content, parse, request},
	helpers::app_language,
	models::{self, Listing, Novel, Title, Update},
	settings,
};
use aidoku::{
	AidokuError, Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link,
	Manga, MangaWithChapter, Result,
	alloc::{String, Vec, format},
	helpers::uri::QueryParameters,
	imports::{
		net::{Request, Response},
		std::send_partial_result,
	},
	prelude::*,
};

#[derive(Clone, Copy)]
pub enum Feed {
	Popular,
	Hot,
	New,
	NewComics,
	Novels,
}

pub const SECTIONS: [(&str, Feed); 5] = [
	("Most Recent Popular", Feed::Popular),
	("Hot Updates", Feed::Hot),
	("New Updates", Feed::New),
	("Most Followed New Comics", Feed::NewComics),
	("Novels", Feed::Novels),
];

fn feed_url(feed: Feed, content: &Content) -> String {
	match feed {
		Feed::Popular => format!("{API_URL}/analytics/popular?window=7d&limit=25&offset=0"),
		Feed::NewComics => format!("{API_URL}/analytics/new-comics?window=7d&limit=25&offset=0"),
		Feed::Novels => format!("{API_URL}/novel"),
		Feed::Hot | Feed::New => {
			let filter = if let Feed::Hot = feed { "hot" } else { "new" };
			let mut query = QueryParameters::new();
			query.push_encoded("limit", Some("50"));
			query.push_encoded("filter", Some(filter));
			content.rating_query(&mut query, true);
			query.push_encoded("page", Some("1"));
			format!("{API_URL}/latest?{query}")
		}
	}
}

// Keep only the first chapter entry per series in site order.
pub fn updates(updates: Vec<Update>, content: &Content) -> Vec<Update> {
	let blocked = settings::blocked_uploaders();
	let mut kept: Vec<Update> = Vec::new();
	for update in updates {
		let uploader = update.uploader().to_lowercase();
		let group = uploader.trim_end_matches(|c: char| c.is_ascii_digit());
		if content
			.languages
			.iter()
			.any(|code| code == app_language(&update.lang))
			&& !blocked
				.iter()
				.any(|name| *name == uploader || name == group)
			&& !kept.iter().any(|other| other.url == update.url)
		{
			kept.push(update);
		}
	}
	kept
}

pub fn fill(
	feed: Feed,
	response: Response,
	content: &Content,
) -> Result<Option<HomeComponentValue>> {
	let row = |entries: Vec<Manga>| {
		(!entries.is_empty()).then(|| HomeComponentValue::Scroller {
			entries: entries.into_iter().map(Link::from).collect(),
			listing: None,
		})
	};
	Ok(match feed {
		Feed::Popular | Feed::NewComics => row(parse::<Listing<Title>>(response)?
			.data
			.into_iter()
			.filter(|title| content.allows(title))
			.filter_map(|title| Manga::try_from(title).ok())
			.collect()),
		Feed::Hot => row(updates(parse::<Listing<Update>>(response)?.data, content)
			.into_iter()
			.filter_map(Update::into_manga)
			.collect()),
		Feed::New => {
			let entries: Vec<_> = updates(parse::<Listing<Update>>(response)?.data, content)
				.into_iter()
				.filter_map(Update::into_chapter)
				.map(|(manga, chapter)| MangaWithChapter { manga, chapter })
				.collect();
			(!entries.is_empty()).then_some(HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries,
				listing: None,
			})
		}
		Feed::Novels if content.allows_novels() => {
			row(parse::<models::Response<Vec<Novel>>>(response)?
				.data
				.into_iter()
				.filter_map(Novel::into_manga)
				.collect())
		}
		Feed::Novels => None,
	})
}

impl Home for Swarm {
	fn get_home(&self) -> Result<HomeLayout> {
		let content = Content::new();
		if content.cleared() {
			return Ok(HomeLayout::default());
		}
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: SECTIONS
				.iter()
				.map(|(title, feed)| HomeComponent {
					title: Some(String::from(*title)),
					subtitle: None,
					value: match feed {
						Feed::New => HomeComponentValue::empty_manga_chapter_list(),
						_ => HomeComponentValue::empty_scroller(),
					},
				})
				.collect(),
		}));

		let requests = SECTIONS
			.iter()
			.map(|(_, feed)| {
				let timeout = match feed {
					Feed::Hot | Feed::New => 2.5,
					_ => 6.0,
				};
				Ok(request(&feed_url(*feed, &content))?.timeout(timeout))
			})
			.collect::<Result<Vec<_>>>()?;
		let mut failure = None;
		let components: Vec<_> = SECTIONS
			.iter()
			.zip(Request::send_all(requests))
			.filter_map(|((title, feed), response)| {
				match response
					.map_err(AidokuError::from)
					.and_then(|response| fill(*feed, response, &content))
				{
					Ok(value) => Some(HomeComponent {
						title: Some(String::from(*title)),
						subtitle: None,
						value: value?,
					}),
					Err(error) => {
						failure = Some(error);
						None
					}
				}
			})
			.collect();
		if components.is_empty() {
			return Err(failure.unwrap_or_else(|| error!("No home sections")));
		}
		Ok(HomeLayout { components })
	}
}
