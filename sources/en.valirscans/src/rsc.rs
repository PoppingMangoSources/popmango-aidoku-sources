//! Helpers for reading values out of a Next.js flight stream.
//!
//! Page data arrives either as raw stream rows (when requested with the `rsc`
//! header) or escaped inside script chunks of a full HTML document, so every
//! lookup is tried against both forms.

use aidoku::{
	alloc::{String, Vec, string::ToString},
	prelude::format,
};
use serde::de::DeserializeOwned;

/// Slices one balanced JSON value starting at `start`.
///
/// The scan tracks string state so braces inside values cannot desync it.
pub fn slice_json(payload: &str, start: usize) -> Option<&str> {
	let mut depth = 0usize;
	let mut in_string = false;
	let mut escaped = false;
	for (index, byte) in payload.bytes().enumerate().skip(start) {
		if escaped {
			escaped = false;
			continue;
		}
		match byte {
			b'\\' => escaped = true,
			b'"' => in_string = !in_string,
			_ if in_string => {}
			b'{' | b'[' => depth += 1,
			b'}' | b']' => {
				depth -= 1;
				if depth == 0 {
					return payload.get(start..=index);
				}
			}
			_ => {}
		}
	}
	None
}

pub fn decode_escaped(payload: &str) -> String {
	payload.replace("\\\"", "\"").replace("\\\\", "\\")
}

/// Text chunks may span newlines, so a row only ends at the next `<id>:` line.
fn until_next_row(text: &str) -> &str {
	let mut offset = 0usize;
	while let Some(found) = text[offset..].find('\n') {
		let index = offset + found;
		let rest = &text[index + 1..];
		let digits = rest.bytes().take_while(u8::is_ascii_hexdigit).count();
		if digits > 0 && rest.as_bytes().get(digits) == Some(&b':') {
			return &text[..index];
		}
		offset = index + 1;
	}
	text
}

fn row_body<'a>(payload: &'a str, id: &str) -> Option<&'a str> {
	let header = format!("{id}:");
	let start = if payload.starts_with(&header) {
		header.len()
	} else {
		payload.find(&format!("\n{header}"))? + header.len() + 1
	};
	let body = payload.get(start..)?;

	// `T<hex length>,` introduces a raw text chunk; every other row is JSON.
	let Some(rest) = body.strip_prefix('T') else {
		return Some(until_next_row(body));
	};
	let (length, text) = rest.split_once(',')?;
	let length = usize::from_str_radix(length.trim(), 16).ok()?;
	Some(text.get(..length).unwrap_or_else(|| until_next_row(text)))
}

/// Resolves a flight reference such as `$59` to the row it points at.
///
/// Long values are not inlined where they are used; the field holds a row id
/// and the payload itself carries the text. Returns `None` for a plain string.
pub fn resolve_reference(payload: &str, value: &str) -> Option<String> {
	let id = value.strip_prefix('$')?;
	// `$L` and `$@` mark lazy and promise references to those same rows.
	let id = id.strip_prefix(['L', '@']).unwrap_or(id);
	if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
		return None;
	}

	let read = |text: &str| {
		let body = row_body(text, id)?;
		Some(match serde_json::from_str::<String>(body) {
			Ok(string) => string,
			Err(_) => body.to_string(),
		})
	};
	read(payload).or_else(|| read(&decode_escaped(payload)))
}

/// Collects every balanced JSON value that follows `marker`.
///
/// With `keep_marker` the slice starts at the marker itself, which is how the
/// site embeds objects like `{"series":...}`.
pub fn extract_all_by_marker<T: DeserializeOwned>(
	payload: &str,
	marker: &str,
	keep_marker: bool,
) -> Vec<T> {
	let mut values: Vec<T> = Vec::new();
	for text in [payload.to_string(), decode_escaped(payload)] {
		let mut offset = 0usize;
		while let Some(found) = text[offset..].find(marker) {
			let index = offset + found;
			let start = if keep_marker {
				index
			} else {
				index + marker.len()
			};
			if matches!(text.as_bytes().get(start), Some(b'{') | Some(b'['))
				&& let Some(raw) = slice_json(&text, start)
				&& let Ok(value) = serde_json::from_str::<T>(raw)
			{
				values.push(value);
			}
			offset = index + marker.len();
		}
		if !values.is_empty() {
			break;
		}
	}
	values
}
