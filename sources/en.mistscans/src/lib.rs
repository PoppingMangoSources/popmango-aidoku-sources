#![no_std]
use aidoku::{Source, prelude::*};
use keyoapp::{Impl, Keyoapp, Params};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://mistscans.com";

struct MistScans;

impl Impl for MistScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			..Default::default()
		}
	}
}

register_source!(Keyoapp<MistScans>, Home, DeepLinkHandler);
