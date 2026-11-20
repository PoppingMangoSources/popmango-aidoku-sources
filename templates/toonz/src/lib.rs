#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeLayout, Listing,
	ListingProvider, Manga, MangaPageResult, Page, Result, Source,
	alloc::{String, Vec},
};

mod helpers;
mod home;
mod imp;
mod models;
mod settings;

pub use imp::Impl;

// A home row, read from the home pages at the given indices.
#[derive(Clone, Copy)]
pub enum Row {
	// Featured articles, highest rated first.
	Featured(&'static [usize]),
	Hero(usize),
	// A section's cards, most recently added first.
	Newest(&'static [usize], &'static str),
	// A section's visible tab.
	Tab(usize, &'static str),
	// A section's cards with their newest chapter.
	Chapters(usize, &'static str),
	// The list view, which names each chapter and its age.
	Updates(usize),
}

pub struct Params {
	pub base_url: &'static str,
	pub catalog: &'static str,
	// The path that resolves every series and chapter.
	pub series_path: &'static str,
	// The paths the site files series under.
	pub series_types: &'static [&'static str],
	pub rating: &'static str,
	// Selecting every type sends none.
	pub type_count: usize,
	pub home_pages: &'static [&'static str],
	pub home_rows: &'static [(&'static str, Row)],
}

pub struct Toonz<T: Impl> {
	inner: T,
	params: Params,
}

impl<T: Impl> Source for Toonz<T> {
	fn new() -> Self {
		let inner = T::new();
		let params = inner.params();
		Self { inner, params }
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		self.inner
			.get_search_manga_list(&self.params, query, page, filters)
	}

	fn get_manga_update(
		&self,
		manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		self.inner
			.get_manga_update(&self.params, manga, needs_details, needs_chapters)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		self.inner.get_page_list(&self.params, manga, chapter)
	}
}

impl<T: Impl> ListingProvider for Toonz<T> {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		self.inner.get_manga_list(&self.params, listing, page)
	}
}

impl<T: Impl> Home for Toonz<T> {
	fn get_home(&self) -> Result<HomeLayout> {
		self.inner.get_home(&self.params)
	}
}

impl<T: Impl> DeepLinkHandler for Toonz<T> {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		self.inner.handle_deep_link(&self.params, url)
	}
}
