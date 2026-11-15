#![no_std]
use aidoku::{Source, prelude::*};
use mangataro::{Impl, MangaTaro, Params};

struct MangaTaroSource;

impl Impl for MangaTaroSource {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: "https://mangataro.org".into(),
			novel_home: false,
		}
	}
}

register_source!(
	MangaTaro<MangaTaroSource>,
	Home,
	DynamicFilters,
	DynamicSettings,
	DeepLinkHandler,
	ImageRequestProvider
);

#[cfg(test)]
mod test;
