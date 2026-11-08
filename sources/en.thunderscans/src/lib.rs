#![no_std]
use aidoku::{HomeLayout, Result, Source, prelude::*};
use mangathemesia::{Impl, MangaThemesia, Params, child_theme};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://en-thunderscans.com";

struct ThunderScans;

impl Impl for ThunderScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			manga_url_directory: "/comics".into(),
			// The banners are 2048×768.
			banner_width: Some(400),
			banner_height: Some(150),
			..Default::default()
		}
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		child_theme::get_home(params)
	}
}

register_source!(
	MangaThemesia<ThunderScans>,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
