#![no_std]
use aidoku::{Source, prelude::*};
use keyoapp::{Impl, Keyoapp, Params};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://genztoons.org";

struct GenzToons;

impl Impl for GenzToons {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			// The site's Pinned Series lists the same series as Latest Updates.
			hidden_rows: &["Pinned Series"],
		}
	}
}

register_source!(Keyoapp<GenzToons>, Home, DeepLinkHandler);
