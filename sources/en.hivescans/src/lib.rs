#![no_std]
use aidoku::{Source, prelude::*};
use iken::{Iken, Impl, Params, home};

mod helpers;
mod models;

const BASE_URL: &str = "https://hivetoons.org";
const API_URL: &str = "https://api.hivetoons.org";

struct HiveScans;

impl Impl for HiveScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			api_url: Some(API_URL.into()),
			..Default::default()
		}
	}
	fn get_home(&self, params: &Params) -> aidoku::Result<aidoku::HomeLayout> {
		helpers::parse_home(&home::fetch(params)?, params)
	}
}

register_source!(Iken<HiveScans>, Home, DeepLinkHandler, ImageRequestProvider);

#[cfg(test)]
mod test;
