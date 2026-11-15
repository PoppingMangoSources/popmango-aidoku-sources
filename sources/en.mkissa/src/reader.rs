use crate::{
	helpers::{manga_url, parse_page_urls},
	models::*,
	settings,
};
use aidoku::{
	Result,
	alloc::{String, Vec},
	imports::{
		js::{WebView, WebViewUserScript},
		net::Request,
	},
	prelude::*,
};

const RESULT_TOKEN: &str = "__AIDOKU_MKISSA_PAGES__";
const CHALLENGE_TOKEN: &str = "__AIDOKU_MKISSA_CHALLENGE__";

// The hidden page only has to route to the chapter, so its images, styles and trackers are skipped
const BLOCKED_CONTENT: &str = r#"[
	{"trigger": {"url-filter": ".*", "resource-type": ["image", "style-sheet", "font", "media"]}, "action": {"type": "block"}},
	{"trigger": {"url-filter": "^https?://[^/]*google-analytics\\.com"}, "action": {"type": "block"}},
	{"trigger": {"url-filter": "^https?://[^/]*googletagmanager\\.com"}, "action": {"type": "block"}},
	{"trigger": {"url-filter": "^https?://[^/]*cloudflareinsights\\.com"}, "action": {"type": "block"}}
]"#;

fn is_parser_key_byte(byte: u8) -> bool {
	byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'$'
}

fn parse_quoted_key(input: &str) -> Option<String> {
	let bytes = input.as_bytes();
	let quote = *bytes.first()?;
	if quote != b'\'' && quote != b'"' {
		return None;
	}
	let end = bytes[1..].iter().position(|byte| *byte == quote)? + 1;
	let key = &input[1..end];
	(!key.is_empty() && key.bytes().all(is_parser_key_byte)).then(|| String::from(key))
}

fn resolve_parser_key_assignment(script: &str, target: &str) -> Option<String> {
	let mut offset = 0;
	while let Some(relative) = script[offset..].find(target) {
		let start = offset + relative;
		let end = start + target.len();
		let before_is_key = start > 0 && is_parser_key_byte(script.as_bytes()[start - 1]);
		let after_is_key = end < script.len() && is_parser_key_byte(script.as_bytes()[end]);
		if !before_is_key && !after_is_key {
			let rest = script[end..].trim_start();
			if rest.starts_with('=') && !rest.starts_with("==") && !rest.starts_with("=>") {
				let expression = rest[1..].split(';').next()?;
				let bytes = expression.as_bytes();
				let mut key = String::new();
				let mut found_literal = false;
				let mut index = 0;
				while index < bytes.len() {
					let quote = bytes[index];
					if quote != b'\'' && quote != b'"' {
						index += 1;
						continue;
					}
					let literal_start = index + 1;
					index = literal_start;
					while index < bytes.len() && bytes[index] != quote {
						index += 1;
					}
					if index >= bytes.len() {
						return None;
					}
					let literal = &expression[literal_start..index];
					if literal.bytes().all(is_parser_key_byte) {
						key.push_str(literal);
						found_literal = true;
					}
					index += 1;
				}
				if found_literal && !key.is_empty() {
					return Some(key);
				}
			}
		}
		offset = end;
	}
	None
}

// The reader pins an untouched JSON.parse under a generated window key, written whole or built from literals
fn pinned_parser_key(html: &str) -> Option<String> {
	let mut offset = 0;
	while let Some(relative) = html[offset..].find("defineProperty") {
		let call_start = offset + relative;
		let script_start = html[..call_start].rfind("<script")?;
		let body_start = script_start + html[script_start..].find('>')? + 1;
		let script_end = call_start + html[call_start..].find("</script>")?;
		let script = &html[body_start..script_end];
		if !script.contains("JSON") {
			offset = call_start + "defineProperty".len();
			continue;
		}

		let after_name = &html[call_start + "defineProperty".len()..script_end];
		let open = after_name.find('(')?;
		let mut rest = after_name[open + 1..].trim_start();
		if !rest.starts_with("window") {
			offset = call_start + "defineProperty".len();
			continue;
		}
		rest = rest["window".len()..].trim_start();
		if !rest.starts_with(',') {
			offset = call_start + "defineProperty".len();
			continue;
		}
		rest = rest[1..].trim_start();

		if let Some(key) = parse_quoted_key(rest) {
			return Some(key);
		}

		let target_len = rest
			.bytes()
			.take_while(|byte| is_parser_key_byte(*byte))
			.count();
		if target_len > 0 {
			let target = &rest[..target_len];
			if let Some(key) = resolve_parser_key_assignment(script, target) {
				return Some(key);
			}
		}
		offset = call_start + "defineProperty".len();
	}
	None
}

// Captures the decoded `chapterPages`, then clicks through to the chapter so the site's router fetches it.
// The promise settles with the pages, the challenge token after five seconds of a check, or empty text.
fn capture_script(chapter_path: &str, pinned_key: Option<&str>) -> String {
	let chapter_path = serde_json::to_string(chapter_path).unwrap_or_else(|_| String::from("\"\""));
	let pin = pinned_key
		.and_then(|key| serde_json::to_string(key).ok())
		.map(|key| {
			format!(
				"try {{
		Object.defineProperty(window, {key}, {{
			value: captureParse,
			writable: false,
			configurable: false,
			enumerable: false
		}});
	}} catch (_) {{}}"
			)
		})
		.unwrap_or_default();

	format!(
		"(function () {{
	if (window['{RESULT_TOKEN}']) return;
	const state = {{ done: false }};
	window['{RESULT_TOKEN}'] = state;
	state.result = new Promise((resolve) => {{
		state.resolve = resolve;
	}});
	const finish = (payload) => {{
		if (state.done) return;
		state.done = true;
		state.resolve(payload);
	}};
	setTimeout(() => finish(''), 30000);

	// The site's own check never clears in a hidden view
	let challenged = 0;
	const watch = setInterval(() => {{
		if (state.done) return clearInterval(watch);
		const challenge = document.title.includes('Just a moment')
			|| document.querySelector('.captcha-overlay--visible') != null;
		challenged = challenge ? challenged + 1 : 0;
		if (challenged >= 20) finish('{CHALLENGE_TOKEN}');
	}}, 250);
	const capture = (parsed, raw) => {{
		try {{
			if (parsed && (parsed.chapterPages || (parsed.data && parsed.data.chapterPages))) {{
				finish(raw);
			}}
		}} catch (_) {{}}
	}};

	const originalJson = Response.prototype.json;
	Response.prototype.json = function () {{
		return originalJson.call(this).then((data) => {{
			capture(data, JSON.stringify(data));
			return data;
		}});
	}};

	const originalParse = JSON.parse;
	const captureParse = new Proxy(originalParse, {{
		apply(target, thisArg, args) {{
			const result = Reflect.apply(target, thisArg, args);
			capture(result, args[0]);
			return result;
		}}
	}});
	JSON.parse = captureParse;
	{pin}

	// The page takes a pristine JSON.parse from a new iframe; with no frame window it keeps ours
	const hideFrame = (element) => {{
		if (element && String(element.tagName).toUpperCase() === 'IFRAME') {{
			Object.defineProperty(element, 'contentWindow', {{ get: () => null, configurable: false }});
		}}
		return element;
	}};
	for (const name of ['createElement', 'createElementNS']) {{
		const create = Document.prototype[name];
		Document.prototype[name] = function (...args) {{
			return hideFrame(create.apply(this, args));
		}};
	}}

	function triggerChapterNav() {{
		const a = document.createElement('a');
		a.href = a.dataset.href = {chapter_path};
		document.body.append(a);
		a.click();
	}}

	let attempts = 0;
	function check() {{
		if (state.done) return;
		if (document.querySelector('[data-href]')) {{
			triggerChapterNav();
		}} else if (attempts < 300) {{
			attempts++;
			setTimeout(check, 50);
		}} else {{
			triggerChapterNav();
		}}
	}}
	check();
}})()"
	)
}

pub fn page_urls(manga_id: &str, chapter: &str) -> Result<Vec<String>> {
	let manga_url = manga_url(manga_id);
	let chapter_path = format!("/manga/{manga_id}/chapter-{chapter}-sub");

	// A plain request lets the app clear Cloudflare and keep its cookie
	let response = Request::get(&manga_url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.header("Accept", "text/html,application/xhtml+xml,*/*;q=0.8")
		.send()?;
	let status = response.status_code();
	if status == 403 || status == 503 {
		bail!("Solve the Cloudflare check, then retry");
	} else if status >= 400 {
		bail!("The reader returned HTTP {status}");
	}
	let html = response.get_string()?;
	let parser_key = pinned_parser_key(&html);

	// Navigating client-side from the cleared page avoids a second Cloudflare check
	let web_view = WebView::new();
	web_view.set_rule_list(BLOCKED_CONTENT)?;
	let mut user_script =
		WebViewUserScript::new(capture_script(&chapter_path, parser_key.as_deref()));
	user_script.at_document_end = false;
	user_script.for_main_frame_only = false;
	web_view.add_user_script(user_script)?;
	web_view.load_html_blocking(&html, Some(&manga_url))?;

	let result = web_view.eval_async(&format!("return await window['{RESULT_TOKEN}'].result;"))?;
	if result == CHALLENGE_TOKEN {
		bail!("Open the series in the web view, solve the site's check, then retry");
	} else if result.is_empty() {
		bail!("The reader did not produce any pages");
	}

	// The page hands back the text it parsed, not a response body
	let payload: ReaderPayload = serde_json::from_str(&result)?;
	let pages = payload
		.chapter_pages
		.or_else(|| payload.data.and_then(|data| data.chapter_pages))
		.ok_or(error!("The reader did not produce any pages"))?;
	parse_page_urls(&pages, &settings::image_quality())
}
