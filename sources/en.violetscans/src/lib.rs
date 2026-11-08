#![no_std]
use aidoku::{
	FilterValue, HomeLayout, MangaPageResult, Result, Source,
	alloc::{String, Vec},
	prelude::*,
};
use mangathemesia::{Impl, MangaThemesia, Params};

mod home;
#[cfg(test)]
mod test;

const BASE_URL: &str = "https://violetmanga.com";

struct VioletScans;

// The template's own search, which VioletScans narrows.
struct TemplateSearch;

impl Impl for TemplateSearch {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params::default()
	}
}

impl Impl for VioletScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			manga_url_directory: "/comics".into(),
			..Default::default()
		}
	}

	// The site lists novels among comics. Their chapters are text, which this source doesn't read,
	// and every novel's title ends in "[Novel]".
	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut result = TemplateSearch.get_search_manga_list(params, query, page, filters)?;
		result
			.entries
			.retain(|manga| !manga.title.ends_with("[Novel]"));
		Ok(result)
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		home::get_home(&params.base_url)
	}
}

register_source!(
	MangaThemesia<VioletScans>,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
