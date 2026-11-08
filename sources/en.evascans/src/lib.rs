#![no_std]
use aidoku::{HomeLayout, Result, Source, prelude::*};
use mangathemesia::{Impl, MangaThemesia, Params, child_theme};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://evascans.net";

struct EvaScans;

impl Impl for EvaScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			manga_url_directory: "/series".into(),
			// The banners are 1983×793.
			banner_width: Some(400),
			banner_height: Some(160),
			..Default::default()
		}
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		child_theme::get_home(params)
	}
}

register_source!(
	MangaThemesia<EvaScans>,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
