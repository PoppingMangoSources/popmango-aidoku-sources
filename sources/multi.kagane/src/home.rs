use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link, Listing,
	ListingKind, MangaWithChapter, Result,
	alloc::{String, Vec},
	imports::{
		net::{Request, Response},
		std::send_partial_result,
	},
};

use crate::{
	Kagane,
	filters::hidden_tag_ids,
	helpers::{SearchParams, parse, sources, sources_request, trending_entries, trending_request},
	models::{SearchResponse, SeriesSummary, UploadSource},
	settings,
};

/// The size the site's own rows ask for.
const ROW_SIZE: i32 = 20;

/// The site's chart shows ten.
const TRENDING_SIZE: i32 = 10;

#[derive(Clone, Copy)]
enum Layout {
	Ranked,
	Row,
	Chapters,
}

/// The popular row's title and sort, from the period picked in settings.
pub fn popular() -> (&'static str, &'static str) {
	match settings::popular_period().as_str() {
		"today" => ("Popular Today", "avg_views_today"),
		"month" => ("Popular This Month", "avg_views_month"),
		"all" => ("Popular All Time", "avg_views"),
		_ => ("Popular This Week", "avg_views_week"),
	}
}

/// The sort behind each listing a home row links to.
pub fn listing_sort(id: &str) -> Option<&'static str> {
	match id {
		"popular" => Some(popular().1),
		"added" => Some("created_at"),
		"updated" => Some("updated_at"),
		_ => None,
	}
}

fn listing(id: &str, name: &str) -> Option<Listing> {
	Some(Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	})
}

fn fill(
	layout: Layout,
	id: &str,
	name: &str,
	response: Response,
	base_url: &str,
	sources: &[UploadSource],
) -> Result<HomeComponentValue> {
	let links = |series: Vec<SeriesSummary>| -> Vec<Link> {
		series
			.into_iter()
			.map(|series| series.into_manga(base_url, sources).into())
			.collect()
	};
	Ok(match layout {
		Layout::Ranked => HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(5),
			entries: links(trending_entries(parse(response)?)),
			listing: listing(id, name),
		},
		Layout::Row => HomeComponentValue::Scroller {
			entries: links(parse::<SearchResponse>(response)?.content),
			listing: listing(id, name),
		},
		Layout::Chapters => HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries: parse::<SearchResponse>(response)?
				.content
				.into_iter()
				.filter_map(|series| series.into_manga_with_chapter(base_url, sources))
				.collect::<Vec<MangaWithChapter>>(),
			listing: listing(id, name),
		},
	})
}

impl Home for Kagane {
	fn get_home(&self) -> Result<HomeLayout> {
		let base_url = settings::base_url();
		let sections = [
			("trending", "Trending This Week", Layout::Ranked),
			("popular", popular().0, Layout::Row),
			("added", "Recently Added", Layout::Row),
			("updated", "Recently Updated", Layout::Chapters),
		];
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: sections
				.iter()
				.map(|(_, name, layout)| HomeComponent {
					title: Some(String::from(*name)),
					subtitle: None,
					value: match layout {
						Layout::Ranked => HomeComponentValue::empty_manga_list(),
						Layout::Row => HomeComponentValue::empty_scroller(),
						Layout::Chapters => HomeComponentValue::empty_manga_chapter_list(),
					},
				})
				.collect(),
		}));

		let hidden_tags = hidden_tag_ids(&base_url)?;
		let mut requests = Vec::new();
		let mut shown = Vec::new();
		for section @ (id, ..) in sections {
			let request = match listing_sort(id) {
				Some(sort) => SearchParams {
					hidden_tags: hidden_tags.clone(),
					..SearchParams::sorted(sort)
				}
				.request(&base_url, 1, ROW_SIZE)?,
				None => trending_request(&base_url, TRENDING_SIZE)?,
			};
			let Some(request) = request else {
				return Ok(HomeLayout::default());
			};
			requests.push(request);
			shown.push(section);
		}
		let show_source = settings::show_source();
		if show_source {
			requests.push(sources_request(&base_url)?);
		}

		let mut responses = Request::send_all(requests);
		let sources = if show_source {
			responses
				.pop()
				.and_then(|response| response.ok())
				.map(sources)
				.unwrap_or_default()
		} else {
			Vec::new()
		};
		for ((id, name, layout), response) in shown.into_iter().zip(responses) {
			let Ok(value) = response
				.map_err(Into::into)
				.and_then(|response| fill(layout, id, name, response, &base_url, &sources))
			else {
				continue;
			};
			send_partial_result(&HomePartialResult::Component(HomeComponent {
				title: Some(String::from(name)),
				subtitle: None,
				value,
			}));
		}

		Ok(HomeLayout::default())
	}
}
