#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{AidokuError, prelude::*};
use aidoku_test::aidoku_test;

struct Site;

impl Impl for Site {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params::default()
	}
}

fn assert_http<T>(result: Result<T>, status: i32) {
	let Err(AidokuError::Message(message)) = result else {
		panic!("expected HTTP {status}");
	};
	assert_eq!(message, format!("HTTP {status}"));
}

// The loopback server is started by scripts/test-http.py.
#[aidoku_test]
#[ignore]
fn review_http_status_and_diagnostics() {
	let Some(base) = option_env!("AIDOKU_HTTP_TEST_URL") else {
		panic!("run scripts/test-http.py");
	};
	let site = Site::new();
	for status in [403, 404, 429, 503] {
		let mut params = site.params();
		params.base_url = format!("{base}/status/{status}").into();
		assert_http(
			site.get_search_manga_list(&params, None, 1, Vec::new()),
			status,
		);
		assert_http(
			site.get_manga_update(
				&params,
				Manga {
					key: "/title/".into(),
					..Default::default()
				},
				true,
				true,
			),
			status,
		);
		assert_http(
			site.get_page_list(
				&params,
				Manga::default(),
				Chapter {
					key: "/chapter/1/".into(),
					..Default::default()
				},
			),
			status,
		);
		assert_http(site.get_home(&params), status);
		assert_http(child_theme::get_home(&params), status);
		let link = child_theme::handle_deep_link(&params, format!("{}/title/", params.base_url));
		if status == 404 {
			assert_eq!(link.unwrap(), None);
		} else {
			assert_http(link, status);
		}
	}
	for status in [200, 201, 204, 299] {
		assert_eq!(
			helpers::fetch(&format!("{base}/status/{status}"))
				.unwrap()
				.status_code(),
			status
		);
	}
}
