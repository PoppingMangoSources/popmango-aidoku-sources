#![no_std]
use aidoku::{Source, prelude::*};
use iken::{Iken, Impl, Params};

const BASE_URL: &str = "https://eternalmangas.org";
const API_URL: &str = "https://api.eternalmangas.org";

struct EternalMangas;

impl Impl for EternalMangas {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			api_url: Some(API_URL.into()),
			new_releases_title: "Nuevos lanzamientos",
			sorts: &["updatedAt", "totalViews", "createdAt", "postTitle"],
			home_slider_selector: ".em-hero .carousel-item".into(),
			home_section_titles: &[
				"Popular Hoy!",
				"Últimas Actualizaciones",
				"Tendencias",
				"Novelas",
			],
		}
	}
}

register_source!(
	Iken<EternalMangas>,
	Home,
	DeepLinkHandler,
	ImageRequestProvider
);

#[cfg(test)]
mod test;
