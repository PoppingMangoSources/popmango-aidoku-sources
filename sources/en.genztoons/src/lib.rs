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
		}
	}
}

register_source!(Keyoapp<GenzToons>, Home, DeepLinkHandler);
