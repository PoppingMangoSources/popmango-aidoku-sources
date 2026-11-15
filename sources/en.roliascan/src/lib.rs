#![no_std]
use aidoku::{Source, prelude::*};
use mangataro::{Impl, MangaTaro, Params};

struct RoliaScan;

impl Impl for RoliaScan {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: "https://roliascan.com".into(),
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
