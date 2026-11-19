use crate::{
	BASE_URL,
	helpers::parse_flight_value,
	models::{ReaderPageProps, ReaderPages},
};
use aidoku::{
	Page, Result,
	alloc::Vec,
	imports::{html::Html, js::WebView, net::Request, std::sleep},
	prelude::*,
};

pub fn pages(chapter_key: &str) -> Result<Vec<Page>> {
	let url = format!("{BASE_URL}/read/{chapter_key}");
	let view = WebView::new();
	view.load(Request::get(&url)?)?;

	// The actual chapter navigation lets the browser handle the site's session.
	let mut html = None;
	let document = format!(
		"location.origin === '{BASE_URL}' && document.readyState !== 'loading' ? document.documentElement.outerHTML : ''"
	);
	for _ in 0..20 {
		if let Ok(value) = view.eval(&document)
			&& !value.is_empty()
		{
			html = Some(value);
			break;
		}
		sleep(1);
	}
	let html =
		html.ok_or_else(|| error!("WebNex did not open the chapter. Open it on webnex.cc."))?;
	let doc = Html::parse_with_url(html, &url)?;
	let Some(props) = parse_flight_value::<ReaderPageProps>(&doc, "{\"chapter\":") else {
		if let Some(message) = doc
			.select_first("section.state .notice-error, body > pre")
			.and_then(|el| el.text())
			.filter(|text| !text.is_empty())
		{
			bail!("{message}");
		}
		bail!("WebNex did not provide the chapter's page list. Open it on webnex.cc.");
	};
	view.eval(&format!(
		"({})({})",
		include_str!("reader.js"),
		serde_json::to_string(&props.pages_url)?
	))?;
	for _ in 0..16 {
		let json = view.eval("window.webnexPageList || ''")?;
		if !json.is_empty() {
			let data: ReaderPages = serde_json::from_str(&json)
				.map_err(|_| error!("WebNex returned an unreadable chapter response"))?;
			return data.try_into();
		}
		sleep(1);
	}
	bail!("WebNex timed out returning the page list. Open the chapter on webnex.cc.")
}
