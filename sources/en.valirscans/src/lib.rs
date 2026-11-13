#![no_std]
use aidoku::{Source, prelude::*};
use vinetheme::{Impl, Params, VineTheme};

const BASE_URL: &str = "https://valirscans.org";

struct ValirScans;

impl Impl for ValirScans {
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
	VineTheme<ValirScans>,
	Home,
	ListingProvider,
	DynamicFilters,
	ImageRequestProvider,
	DeepLinkHandler
);

#[cfg(test)]
mod test;
