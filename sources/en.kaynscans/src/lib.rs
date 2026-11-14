#![no_std]
use aidoku::{Source, prelude::*};
use vinetheme::{Impl, Params, VineTheme};

const BASE_URL: &str = "https://kaynscans.com";

struct KaynScans;

impl Impl for KaynScans {
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
	VineTheme<KaynScans>,
	Home,
	ListingProvider,
	DynamicFilters,
	ImageRequestProvider,
	DeepLinkHandler
);
