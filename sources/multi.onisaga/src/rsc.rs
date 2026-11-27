use aidoku::{
	alloc::{String, collections::BTreeMap},
	imports::html::Document,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

enum Row<'a> {
	Text(&'a str),
	Json(Value),
}

// Next.js pushes its page data as JSON strings of `id:row` lines.
pub fn extract<T: DeserializeOwned>(doc: &Document, keys: &[&str]) -> Option<T> {
	let mut flight = String::new();
	for script in doc.select("script").into_iter().flatten() {
		if let Some(data) = script.data().or_else(|| script.html())
			&& let Some(json) = data.trim().strip_prefix("self.__next_f.push([1,")
			&& let Some(Ok(chunk)) = serde_json::Deserializer::from_str(json)
				.into_iter::<String>()
				.next()
		{
			flight.push_str(&chunk);
		}
	}
	let rows = rows(&flight);
	let found = rows.values().find_map(|row| match row {
		Row::Json(value) => find(value, keys),
		Row::Text(_) => None,
	})?;
	serde_json::from_value(resolve(found, &rows)).ok()
}

fn rows(flight: &str) -> BTreeMap<&str, Row<'_>> {
	let mut rows = BTreeMap::new();
	let mut remaining = flight;
	while let Some((id, row)) = remaining.split_once(':') {
		if id.is_empty() || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
			remaining = remaining.split_once('\n').map_or("", |(_, rest)| rest);
			continue;
		}
		if let Some(raw) = row.strip_prefix('T')
			&& let Some((length, text)) = raw.split_once(',')
			&& let Ok(length) = usize::from_str_radix(length, 16)
			&& let (Some(value), Some(rest)) = (text.get(..length), text.get(length..))
		{
			// Byte-length text rows may contain newlines or touch the next row.
			rows.insert(id, Row::Text(value));
			remaining = rest.trim_start_matches('\n');
			continue;
		}
		let (line, rest) = row.split_once('\n').unwrap_or((row, ""));
		if let Ok(value) = serde_json::from_str(line) {
			rows.insert(id, Row::Json(value));
		}
		remaining = rest;
	}
	rows
}

fn find<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Value> {
	match value {
		Value::Object(object) if keys.iter().all(|key| object.contains_key(*key)) => Some(value),
		Value::Object(object) => object.values().find_map(|value| find(value, keys)),
		Value::Array(values) => values.iter().find_map(|value| find(value, keys)),
		_ => None,
	}
}

// Long strings and repeated objects are sent once, then pointed at as `$<row>` or
// `$<row>:<path>`.
fn resolve(value: &Value, rows: &BTreeMap<&str, Row<'_>>) -> Value {
	match value {
		Value::String(text) => reference(text, rows).unwrap_or_else(|| value.clone()),
		Value::Array(values) => {
			Value::Array(values.iter().map(|value| resolve(value, rows)).collect())
		}
		Value::Object(object) => Value::Object(
			object
				.iter()
				.map(|(key, value)| (key.clone(), resolve(value, rows)))
				.collect(),
		),
		_ => value.clone(),
	}
}

fn reference(text: &str, rows: &BTreeMap<&str, Row<'_>>) -> Option<Value> {
	let mut path = text.strip_prefix('$')?.split(':');
	let id = path
		.next()
		.filter(|id| !id.is_empty() && id.bytes().all(|byte| byte.is_ascii_hexdigit()))?;
	let mut value = match rows.get(id)? {
		Row::Text(text) => return Some(Value::String((*text).into())),
		Row::Json(value) => value,
	};
	for segment in path {
		value = match value {
			// An element is `["$", type, key, props]`.
			Value::Array(element) if segment == "props" => element.get(3)?,
			Value::Array(values) => values.get(segment.parse::<usize>().ok()?)?,
			Value::Object(object) => object.get(segment)?,
			_ => return None,
		};
	}
	Some(resolve(value, rows))
}
