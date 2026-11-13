use crate::{
	MangaBall,
	api::{API_URL, SORTS, SearchParams, parse, request},
	models::{ApiResponse, Title},
};
use aidoku::{
	Home, HomeComponent, HomeComponentValue, HomeLayout, HomePartialResult, Link, Listing,
	ListingKind, Manga, MangaWithChapter, Result,
	alloc::{String, Vec, format},
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

#[derive(Clone, Copy)]
pub enum Layout {
	Featured,
	Ranked,
	Row,
	Chapters,
}

pub const SECTIONS: [(&str, &str, Layout); 5] = [
	("featured", "Featured Manga", Layout::Featured),
	("top-views", "Top Weekly Ranking", Layout::Ranked),
	("recommended", "Titles Recommended", Layout::Row),
	("lastupdate", "Latest Chapter Updates", Layout::Chapters),
	("created_at", "Recently Added Titles", Layout::Row),
];

pub fn section_url(id: &'static str, search: &SearchParams) -> String {
	let adult_mode = search.adult_mode;
	match id {
		"featured" | "recommended" => {
			format!("{API_URL}/title/{id}?limit=10&adult_mode={adult_mode}")
		}
		"top-views" => format!(
			"{API_URL}/title/top-views?period=week&limit=9&section_key=sidebar_ranking&adult_mode={adult_mode}"
		),
		_ => SearchParams::sorted(id).url(1),
	}
}

pub fn fill(
	id: &str,
	name: &str,
	layout: Layout,
	titles: Vec<Title>,
	search: &SearchParams,
) -> HomeComponentValue {
	// Search applies the settings itself.
	let sorted = SORTS.contains(&id);
	let titles = titles
		.into_iter()
		.filter(|title| sorted || search.allows(title));
	let listing = sorted.then(|| Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	});
	let link = |title: Title| Link::from(Manga::from(title));
	match layout {
		Layout::Featured => HomeComponentValue::BigScroller {
			entries: titles.map(Title::into_details).collect(),
			auto_scroll_interval: Some(5.0),
		},
		Layout::Ranked => HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(3),
			entries: titles.map(link).collect(),
			listing,
		},
		Layout::Row => HomeComponentValue::Scroller {
			entries: titles.map(link).collect(),
			listing,
		},
		Layout::Chapters => HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries: titles
				.filter_map(|mut title| {
					let chapter = core::mem::take(&mut title.recent_chapters)
						.into_iter()
						.find_map(|chapter| chapter.into_chapter(&search.languages))?;
					Some(MangaWithChapter {
						manga: title.into(),
						chapter,
					})
				})
				.collect(),
			listing,
		},
	}
}

impl Home for MangaBall {
	fn get_home(&self) -> Result<HomeLayout> {
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: SECTIONS
				.iter()
				.map(|(_, name, layout)| HomeComponent {
					title: Some(String::from(*name)),
					subtitle: None,
					value: match layout {
						Layout::Featured => HomeComponentValue::empty_big_scroller(),
						Layout::Ranked => HomeComponentValue::empty_manga_list(),
						Layout::Row => HomeComponentValue::empty_scroller(),
						Layout::Chapters => HomeComponentValue::empty_manga_chapter_list(),
					},
				})
				.collect(),
		}));

		let search = SearchParams::new(None);
		let requests = SECTIONS
			.iter()
			.map(|(id, ..)| request(&section_url(id, &search)))
			.collect::<Result<Vec<_>>>()?;
		// Returning the final layout removes failed-section placeholders.
		let components = SECTIONS
			.iter()
			.zip(Request::send_all(requests))
			.filter_map(|((id, name, layout), response)| {
				let titles = parse::<ApiResponse<Vec<Title>>>(response.ok()?).ok()?.data;
				Some(HomeComponent {
					title: Some(String::from(*name)),
					subtitle: None,
					value: fill(id, name, *layout, titles, &search),
				})
			})
			.collect::<Vec<_>>();
		if components.is_empty() {
			bail!("MangaBall didn't answer");
		}
		Ok(HomeLayout { components })
	}
}
