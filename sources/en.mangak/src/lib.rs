#![no_std]
use aidoku::{Source, prelude::*};
use mangak::{Impl, Params};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://mangak.io";
const API_URL: &str = "https://api.mangak.io";

struct MangaK;

impl Impl for MangaK {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			api_url: API_URL.into(),
			featured_title: "Hot this week".into(),
			featured_path: "trending/titles?window=week&limit=5".into(),
			latest_title: "Recently Updated".into(),
			trending_today_section: true,
		}
	}
}

register_source!(
	mangak::MangaK<MangaK>,
	ListingProvider,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
