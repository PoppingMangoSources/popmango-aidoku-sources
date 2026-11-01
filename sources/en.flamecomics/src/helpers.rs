use crate::{
	BASE_URL,
	models::{ChapterItem, NextData, Unlock},
};
use aidoku::{
	Result,
	alloc::{String, Vec, collections::BTreeSet, string::ToString},
	imports::{html::Html, net::Request},
	prelude::*,
};
use serde::de::DeserializeOwned;

/// Every page embeds its data as JSON. Reading the page itself takes one request, where the
/// site's data route would need the build id from another page first.
pub fn fetch_props<T: DeserializeOwned>(path: &str) -> Result<T> {
	let response = Request::get(format!("{BASE_URL}{path}"))?.send()?;
	if response.status_code() == 404 {
		bail!("This title is no longer on Flame Comics");
	}
	let json = response
		.get_html()?
		.select_first("script#__NEXT_DATA__")
		.and_then(|script| script.html())
		.ok_or_else(|| error!("No page data found at {path}"))?;
	serde_json::from_str::<NextData<T>>(&json)
		.map(|data| data.props.page_props)
		.map_err(|err| error!("Unexpected page data at {path}: {err}"))
}

/// The text of each non-empty paragraph in an html fragment, or all of its text when it has
/// no paragraphs.
pub fn paragraphs(html: &str) -> Vec<String> {
	let Ok(document) = Html::parse_fragment(html) else {
		return Vec::new();
	};
	let mut texts: Vec<String> = document
		.select("p")
		.map(|list| list.filter_map(|paragraph| paragraph.text()).collect())
		.unwrap_or_default();
	if texts.is_empty() {
		texts.extend(document.select_first("body").and_then(|body| body.text()));
	}
	texts
		.into_iter()
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty())
		.collect()
}

/// The site's own early-access rule: a priced chapter is public once its release time
/// passes, or, under a rolling policy, once it is no longer among the newest few.
pub fn locked_chapters(unlock: &Unlock, chapters: &[ChapterItem], now: i64) -> Vec<bool> {
	let rolling = unlock.unlock_policy.as_deref() == Some("ROLLING_LATEST");
	let mut count = unlock
		.locked_chapter_count
		.filter(|count| *count > 0)
		.unwrap_or(10) as usize;
	if let Some(percent) = unlock.locked_chapter_percent.filter(|p| *p > 0) {
		let share = (chapters.len() * percent.min(100) as usize).div_ceil(100);
		count = count.max(share);
		if let Some(max) = unlock.locked_chapter_max.filter(|max| *max > 0) {
			count = count.min(max.min(100) as usize);
		}
	}
	let mut numbers: Vec<f32> = chapters.iter().filter_map(ChapterItem::number).collect();
	numbers.sort_by(|a, b| b.total_cmp(a));
	let newest: BTreeSet<u32> = numbers.iter().take(count).map(|n| n.to_bits()).collect();

	chapters
		.iter()
		.map(|chapter| {
			let priced = chapter
				.price
				.as_deref()
				.and_then(|price| price.parse::<f32>().ok())
				.is_some_and(|price| price > 0.0);
			let public = match chapter.public_release.filter(|time| *time > 0) {
				Some(time) => now >= time,
				None if rolling => !chapter
					.number()
					.is_some_and(|number| newest.contains(&number.to_bits())),
				None => true,
			};
			priced && !public
		})
		.collect()
}
