use crate::{BASE_URL, models::ReaderPages};
use aidoku::{
	Page, Result,
	alloc::Vec,
	imports::{
		js::{WebView, WebViewUserScript},
		net::Request,
		std::sleep,
	},
	prelude::*,
};

pub fn pages(chapter_key: &str) -> Result<Vec<Page>> {
	let url = format!("{BASE_URL}/read/{chapter_key}");
	let view = WebView::new();
	// Start reading in the chapter's session without a round trip for the HTML.
	view.add_user_script(WebViewUserScript {
		source: include_str!("reader.js").into(),
		at_document_end: true,
		for_main_frame_only: true,
	})?;
	view.load(Request::get(&url)?)?;
	for _ in 0..32 {
		if let Ok(json) = view.eval("window.webnexPageList || ''")
			&& !json.is_empty()
		{
			let data: ReaderPages = serde_json::from_str(&json)
				.map_err(|_| error!("WebNex returned an unreadable chapter response"))?;
			return data.try_into();
		}
		sleep(1);
	}
	bail!("WebNex timed out returning the page list. Open the chapter on webnex.cc.")
}
