use aidoku::alloc::{String, string::ToString};
use serde::de::DeserializeOwned;

/// Finds the first occurrence of `needle` in `haystack` at or after `from`.
fn find_bytes(haystack: &[u8], needle: &[u8], from: usize) -> Option<usize> {
	if needle.is_empty() || from >= haystack.len() {
		return None;
	}
	haystack[from..]
		.windows(needle.len())
		.position(|window| window == needle)
		.map(|pos| from + pos)
}

/// The route ships its data as escaped string fragments pushed onto
/// `self.__next_f`; they only form valid JSON once concatenated in order. A
/// payload served directly for an rsc request arrives already unwrapped.
fn decode_flight(body: &str) -> String {
	let bytes = body.as_bytes();
	let needle = b"self.__next_f.push([1,\"";
	let mut pieces = String::new();
	let mut found_any = false;
	let mut cursor = 0;
	while let Some(pos) = find_bytes(bytes, needle, cursor) {
		let start = pos + needle.len();
		let mut j = start;
		let mut escaped = false;
		while j < bytes.len() {
			let c = bytes[j];
			if escaped {
				escaped = false;
			} else if c == b'\\' {
				escaped = true;
			} else if c == b'"' {
				break;
			}
			j += 1;
		}
		let mut quoted = String::with_capacity(j - start + 2);
		quoted.push('"');
		quoted.push_str(&body[start..j]);
		quoted.push('"');
		if let Ok(serde_json::Value::String(text)) =
			serde_json::from_str::<serde_json::Value>(&quoted)
		{
			pieces.push_str(&text);
			found_any = true;
		}
		cursor = j + 1;
	}
	if found_any { pieces } else { body.to_string() }
}

/// Returns the balanced `{...}` object beginning at `start`, respecting string
/// literals and escapes.
fn balanced_object(text: &str, start: usize) -> Option<&str> {
	let bytes = text.as_bytes();
	let mut depth = 0i32;
	let mut in_string = false;
	let mut escaped = false;
	let mut i = start;
	while i < bytes.len() {
		let c = bytes[i];
		if escaped {
			escaped = false;
		} else if c == b'\\' {
			if in_string {
				escaped = true;
			}
		} else if c == b'"' {
			in_string = !in_string;
		} else if !in_string {
			if c == b'{' {
				depth += 1;
			} else if c == b'}' {
				depth -= 1;
				if depth == 0 {
					return Some(&text[start..=i]);
				}
			}
		}
		i += 1;
	}
	None
}

/// Walks back from the key to the nearest object start that parses cleanly and
/// still contains it, so the surrounding component tree is skipped.
pub fn extract_flight<T: DeserializeOwned>(body: &str, key: &str) -> Option<T> {
	let blob = decode_flight(body);
	let marker = {
		let mut m = String::with_capacity(key.len() + 3);
		m.push('"');
		m.push_str(key);
		m.push_str("\":");
		m
	};
	let bytes = blob.as_bytes();
	let mut from = blob.find(&marker);
	while let Some(f) = from {
		let mut start = f as isize;
		while start >= 0 {
			if bytes[start as usize] == b'{'
				&& let Some(slice) = balanced_object(&blob, start as usize)
				&& slice.len() >= marker.len()
				&& slice.contains(&marker)
				&& let Ok(value) = serde_json::from_str::<T>(slice)
			{
				return Some(value);
			}
			start -= 1;
		}
		from = blob[f + marker.len()..]
			.find(&marker)
			.map(|p| f + marker.len() + p);
	}
	None
}
