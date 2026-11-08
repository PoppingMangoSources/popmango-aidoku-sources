#![no_std]
use aidoku::{
	Chapter, Manga, Page, PageContent, Result, Source,
	alloc::{String, Vec},
	imports::net::Request,
	prelude::*,
};
use base64::{Engine, engine::general_purpose::STANDARD};
use mangathemesia::{Impl, MangaThemesia, Params, helpers::extract_images};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://scythescans.com";

struct ScytheScans;

impl Impl for ScytheScans {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			..Default::default()
		}
	}

	// The site's cache plugin moves the reader's image list into a base64 `data:` script, where the
	// template's search for it can't see.
	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let html = Request::get(format!("{}{}", params.base_url, chapter.key))?.string()?;
		let images = html
			.split("src=\"data:text/javascript;base64,")
			.skip(1)
			.filter_map(|part| STANDARD.decode(part.split('"').next()?).ok())
			.filter_map(|script| String::from_utf8(script).ok())
			.find(|script| script.contains("\"images\":"))
			.map(|script| extract_images(&script))
			.unwrap_or_default();
		if images.is_empty() {
			bail!("No pages found");
		}
		Ok(images
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}
}

register_source!(
	MangaThemesia<ScytheScans>,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
