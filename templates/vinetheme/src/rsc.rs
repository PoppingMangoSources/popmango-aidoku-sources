use aidoku::{
	Result,
	alloc::{String, Vec, borrow::Cow},
	prelude::*,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn decode(body: &str) -> Result<Cow<'_, str>> {
	if !body.trim_start().starts_with('<') {
		return Ok(Cow::Borrowed(body));
	}
	let mut flight = String::new();
	for chunk in body.split("self.__next_f.push(").skip(1) {
		let Some(Ok(value)) = serde_json::Deserializer::from_str(chunk)
			.into_iter::<Value>()
			.next()
		else {
			continue;
		};
		if value[0] == 1
			&& let Some(text) = value[1].as_str()
		{
			flight.push_str(text);
		}
	}
	if flight.is_empty() {
		bail!("Missing page data");
	}
	Ok(Cow::Owned(flight))
}

pub fn extract<T: DeserializeOwned>(body: &str, keys: &[&str]) -> Result<T> {
	let value = objects(body, keys)
		.into_iter()
		.next()
		.ok_or(error!("Missing {} in page data", keys.join("/")))?;
	Ok(serde_json::from_value(value)?)
}

pub fn objects(body: &str, keys: &[&str]) -> Vec<Value> {
	let mut output = Vec::new();
	let mut remaining = body;
	while let Some((id, row)) = remaining.split_once(':') {
		if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
			remaining = remaining.split_once('\n').map_or("", |(_, rest)| rest);
			continue;
		}
		if let Some(raw) = row.strip_prefix('T') {
			let Some((length, text)) = raw.split_once(',') else {
				break;
			};
			let Some(length) = usize::from_str_radix(length, 16).ok() else {
				break;
			};
			// Text rows have a byte length and need not end at a newline.
			let Some(rest) = text.get(length..) else {
				break;
			};
			remaining = rest.trim_start_matches('\n');
		} else {
			let (line, rest) = row.split_once('\n').unwrap_or((row, ""));
			if let Ok(value) = serde_json::from_str(line) {
				collect(value, keys, &mut output);
			}
			remaining = rest;
		}
	}
	output
}

fn collect(value: Value, keys: &[&str], output: &mut Vec<Value>) {
	match value {
		Value::Object(object) if keys.iter().all(|key| object.contains_key(*key)) => {
			output.push(Value::Object(object))
		}
		Value::Object(object) => {
			for (_, value) in object {
				collect(value, keys, output);
			}
		}
		Value::Array(values) => {
			for value in values {
				collect(value, keys, output);
			}
		}
		_ => {}
	}
}

pub fn text<'a>(body: &'a str, value: &'a str) -> Result<Cow<'a, str>> {
	let Some(id) = value
		.strip_prefix('$')
		.filter(|id| !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()))
	else {
		return Ok(Cow::Borrowed(value));
	};
	let marker = format!("\n{id}:");
	let row = body
		.split_once(&marker)
		.map(|(_, row)| row)
		.or_else(|| body.strip_prefix(&marker[1..]))
		.ok_or(error!("Missing chapter text"))?;
	if let Some(raw) = row.strip_prefix('T') {
		let (length, raw) = raw.split_once(',').ok_or(error!("Invalid chapter text"))?;
		let length =
			usize::from_str_radix(length, 16).map_err(|_| error!("Invalid chapter text length"))?;
		return Ok(Cow::Borrowed(
			raw.get(..length).ok_or(error!("Incomplete chapter text"))?,
		));
	}
	Ok(Cow::Owned(serde_json::from_str(
		row.lines().next().unwrap_or_default(),
	)?))
}
