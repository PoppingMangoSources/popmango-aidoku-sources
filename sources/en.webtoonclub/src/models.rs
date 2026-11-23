use crate::BASE_URL;
use aidoku::{
	AidokuError, Page, PageContent, Result,
	alloc::{String, format},
	prelude::*,
};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderImage {
	filename: String,
	#[serde(default)]
	pub sort_order: u32,
}

impl TryFrom<ReaderImage> for Page {
	type Error = AidokuError;

	fn try_from(image: ReaderImage) -> Result<Self> {
		let filename = image.filename.trim().trim_start_matches('/');
		if filename.is_empty() {
			bail!("This chapter has missing pages");
		}
		Ok(Self {
			content: PageContent::url(format!("{BASE_URL}/uploads/{filename}")),
			..Default::default()
		})
	}
}
