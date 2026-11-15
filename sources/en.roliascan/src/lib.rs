#![no_std]
use aidoku::{Source, prelude::*};
use mangataro::{Impl, MangaTaro, Params};

const BASE_URL: &str = "https://roliascan.com";

struct RoliaScan;

impl Impl for RoliaScan {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			// Its Recently Added grid repeats Latest Updates
			recently_added: false,
			novel_home: true,
		}
	}
}

register_source!(
	MangaTaro<RoliaScan>,
	Home,
	DynamicFilters,
	DynamicSettings,
	DeepLinkHandler,
	ImageRequestProvider
);

#[cfg(test)]
mod test;
