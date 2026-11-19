#![no_std]
use aidoku::{Source, prelude::*};
use toonz::{Impl, Params, Row, Toonz};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://toonz.to";

const MANHWA: usize = 0;
const MANGA: usize = 1;

struct ToonzTo;

impl Impl for ToonzTo {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL,
			catalog: "comics",
			series_path: "comic",
			// `/comic/` redirects to the series' type.
			series_types: &["manhwa", "manga", "manhua", "webtoon", "comic", "western"],
			rating: "erotica",
			type_count: 5,
			// The manhwa and manga home pages, then the all-type list, which shows update times.
			home_pages: &["/manhwa", "/manga", "/comics?sort=latest&view=list"],
			home_rows: &[
				("Top Rated This Month", Row::Featured(&[MANHWA, MANGA])),
				("New Titles", Row::Newest(&[MANHWA, MANGA], "New Titles")),
				("Recently Updated", Row::Updates(2)),
				(
					"Popular Updates (Manga)",
					Row::Tab(MANGA, "Popular Updates"),
				),
				(
					"Popular Updates (Manhwa)",
					Row::Tab(MANHWA, "Popular Updates"),
				),
				("Trending Manga", Row::Tab(MANGA, "Trending")),
				("Trending Manhwa", Row::Tab(MANHWA, "Trending")),
			],
		}
	}
}

register_source!(Toonz<ToonzTo>, Home, ListingProvider, DeepLinkHandler);
