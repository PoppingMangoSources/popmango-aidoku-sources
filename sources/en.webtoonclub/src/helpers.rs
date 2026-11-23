use crate::models::ReaderImage;
use aidoku::{Page, Result, alloc::Vec, prelude::*};

pub fn parse_page_list(body: &str) -> Result<Vec<Page>> {
	// The first "images" array that reads as pages; another can come first.
	let marker = r#""images":"#;
	let mut images = body
		.match_indices(marker)
		.find_map(|(start, _)| {
			serde_json::Deserializer::from_str(&body[start + marker.len()..])
				.into_iter::<Vec<ReaderImage>>()
				.next()?
				.ok()
				.filter(|images| !images.is_empty())
		})
		.ok_or_else(|| error!("This chapter has missing pages"))?;
	// Stable, so missing or equal positions keep the response order.
	images.sort_by_key(|image| image.sort_order);
	images.into_iter().map(Page::try_from).collect()
}
