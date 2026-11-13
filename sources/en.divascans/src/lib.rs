#![no_std]
use aidoku::{Source, prelude::*};
use vinetheme::{Impl, Params, VineTheme};

const BASE_URL: &str = "https://divascans.org";

struct DivaScans;

impl Impl for DivaScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
		}
	}
}

register_source!(
	VineTheme<DivaScans>,
	Home,
	ListingProvider,
	DynamicFilters,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
