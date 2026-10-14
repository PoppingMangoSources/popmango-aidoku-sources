use crate::{
	Toonkotsu,
	api::{self, BrowseParams, browse_request, discovery_request, latest_request, ranking_request},
	helpers::allows,
	models::{LatestList, Series},
};
use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing, ListingKind,
	MangaWithChapter, Result,
	alloc::{Vec, vec},
	imports::net::{Request, RequestError, Response},
};

/// The home sections, as the id the listing opens with and the title above it.
const SECTIONS: &[(&str, &str)] = &[
	("views", "Most Read"),
	("trending", "Trending"),
	("score", "Top Rated"),
	("gems", "Hidden Gems"),
	("new", "Recently Added"),
	("seasonal", "This Season"),
	("completed", "Completed"),
];

/// Reading rankings come from the site's own readers, the curated ones are posted, and
/// everything else is browse with a sort or a set of the site's own.
pub fn section_request(id: &str, page: i32) -> Result<Request> {
	match id {
		"views" | "trending" => ranking_request(id, &BrowseParams::new("latest", page)),
		"gems" => discovery_request(id, page),
		"seasonal" | "completed" => {
			let mut params = BrowseParams::new("latest", page);
			params.discovery = id.into();
			browse_request(&params)
		}
		sort => browse_request(&BrowseParams::new(sort, page)),
	}
}

fn section_title(id: &str) -> Option<&'static str> {
	SECTIONS
		.iter()
		.find_map(|(section, title)| (*section == id).then_some(*title))
}

fn links(response: Response) -> Result<Vec<Link>> {
	Ok(api::series_page(response, 1, Series::into_manga)?
		.entries
		.into_iter()
		.map(Link::from)
		.collect())
}

fn listing(id: &str) -> Option<Listing> {
	Some(Listing {
		id: id.into(),
		name: section_title(id)?.into(),
		kind: ListingKind::Default,
	})
}

fn scroller(id: &str, response: Response) -> Result<HomeComponent> {
	Ok(HomeComponent {
		title: section_title(id).map(Into::into),
		value: HomeComponentValue::Scroller {
			entries: links(response)?,
			listing: listing(id),
		},
		..Default::default()
	})
}

impl Home for Toonkotsu {
	fn get_home(&self) -> Result<HomeLayout> {
		let responses: [core::result::Result<Response, RequestError>; 8] = Request::send_all([
			section_request("views", 1)?,
			section_request("trending", 1)?,
			section_request("score", 1)?,
			section_request("gems", 1)?,
			latest_request(1)?,
			section_request("new", 1)?,
			section_request("seasonal", 1)?,
			section_request("completed", 1)?,
		])
		.try_into()
		.expect("requests vec length should be 8");
		let [
			most_read,
			trending,
			top_rated,
			gems,
			latest,
			recent,
			seasonal,
			completed,
		] = responses;

		// The feed lists every new chapter, where the site shows each title once, at its newest.
		let mut latest_entries: Vec<MangaWithChapter> = Vec::new();
		for item in api::parse::<LatestList>(latest?)?.items.unwrap_or_default() {
			if !allows(&item.series) {
				continue;
			}
			let Some(manga) = item.series.into_manga() else {
				continue;
			};
			if latest_entries
				.iter()
				.any(|entry| entry.manga.key == manga.key)
			{
				continue;
			}
			latest_entries.push(MangaWithChapter {
				manga,
				chapter: item.chapter.into_chapter(),
			});
		}

		Ok(HomeLayout {
			components: vec![
				HomeComponent {
					title: section_title("views").map(Into::into),
					// The big scroller is the one component that renders a description and tags.
					value: HomeComponentValue::BigScroller {
						entries: api::series_page(most_read?, 1, Series::into_full_manga)?
							.entries
							.into_iter()
							.take(10)
							.collect(),
						auto_scroll_interval: None,
					},
					..Default::default()
				},
				HomeComponent {
					title: section_title("trending").map(Into::into),
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(5),
						entries: links(trending?)?,
						listing: listing("trending"),
					},
					..Default::default()
				},
				scroller("score", top_rated?)?,
				scroller("gems", gems?)?,
				HomeComponent {
					title: Some("Latest Updates".into()),
					value: HomeComponentValue::MangaChapterList {
						page_size: Some(5),
						entries: latest_entries,
						listing: None,
					},
					..Default::default()
				},
				scroller("new", recent?)?,
				scroller("seasonal", seasonal?)?,
				scroller("completed", completed?)?,
			],
		})
	}
}
