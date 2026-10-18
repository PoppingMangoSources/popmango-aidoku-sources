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
	helpers::{SearchParams, parse, sources, sources_request},
	models::{SearchResponse, SeriesSummary, UploadSource},
	settings,
};

/// The size the site's own rows ask for.
const ROW_SIZE: i32 = 20;

#[derive(Clone, Copy)]
enum Layout {
	Row,
	Chapters,
}

/// Home rows: listing id, title, sort and layout. The popular row's period is a setting.
fn sections() -> [(&'static str, &'static str, &'static str, Layout); 3] {
	let (popular_name, popular_sort) = match settings::popular_period().as_str() {
		"today" => ("Popular Today", "avg_views_today"),
		"month" => ("Popular This Month", "avg_views_month"),
		"all" => ("Popular All Time", "avg_views"),
		_ => ("Popular This Week", "avg_views_week"),
	};
	[
		("popular", popular_name, popular_sort, Layout::Row),
		("added", "Recently Added", "created_at", Layout::Row),
		(
			"updated",
			"Recently Updated",
			"updated_at",
			Layout::Chapters,
		),
	]
}

pub fn listing_sort(id: &str) -> Option<&'static str> {
	sections()
		.into_iter()
		.find(|(section, ..)| *section == id)
		.map(|(_, _, sort, _)| sort)
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
		let sections = sections();
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: sections
				.iter()
				.map(|(_, name, _, layout)| HomeComponent {
					title: Some(String::from(*name)),
					subtitle: None,
					value: match layout {
						Layout::Row => HomeComponentValue::empty_scroller(),
						Layout::Chapters => HomeComponentValue::empty_manga_chapter_list(),
					},
				})
				.collect(),
		}));

		let hidden_tags = hidden_tag_ids(&base_url)?;
		let mut requests = Vec::new();
		for (_, _, sort, _) in sections {
			let request = SearchParams {
				hidden_tags: hidden_tags.clone(),
				..SearchParams::sorted(sort)
			}
			.request(&base_url, 1, ROW_SIZE)?;
			let Some(request) = request else {
				return Ok(HomeLayout::default());
			};
			requests.push(request);
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
		for ((id, name, _, layout), response) in sections.into_iter().zip(responses) {
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
