use aidoku::{
	alloc::{String, Vec, collections::BTreeMap},
	imports::html::Document,
};
use serde_json::{Map, Value};

const PUSH: &str = "self.__next_f.push(";

pub struct Flight {
	rows: Vec<Value>,
	modules: BTreeMap<String, Value>,
}

impl Flight {
	pub fn from_document(doc: &Document) -> Self {
		let mut payload = String::new();
		for script in doc.select("script").into_iter().flatten() {
			if let Some(code) = script.data().or_else(|| script.text()) {
				append_chunks(&code, &mut payload);
			}
		}
		let (rows, modules) = parse_rows(&payload);
		Self { rows, modules }
	}

	pub fn find(&self, keys: &[&str]) -> Option<&Map<String, Value>> {
		self.rows.iter().find_map(|row| search(row, keys))
	}

	pub fn all(&self, keys: &[&str]) -> Vec<&Map<String, Value>> {
		let mut found = Vec::new();
		for row in &self.rows {
			collect(row, keys, &mut found);
		}
		found
	}

	pub fn module(&self, id: &str) -> Option<&Value> {
		self.modules.get(id)
	}

	pub fn reference_of(&self, props: &Map<String, Value>) -> Option<&str> {
		self.rows.iter().find_map(|row| reference(row, props))
	}

	pub fn all_arrays(&self) -> Vec<&Vec<Value>> {
		let mut arrays = Vec::new();
		for row in &self.rows {
			collect_arrays(row, &mut arrays);
		}
		arrays
	}
}

fn append_chunks(script: &str, payload: &mut String) {
	let mut rest = script;
	while let Some(at) = rest.find(PUSH) {
		let args = &rest[at + PUSH.len()..];
		let mut values = serde_json::Deserializer::from_str(args).into_iter::<Value>();
		match values.next() {
			Some(Ok(Value::Array(items))) => {
				if let [Value::Number(kind), Value::String(chunk), ..] = items.as_slice()
					&& kind.as_u64() == Some(1)
				{
					payload.push_str(chunk);
				}
				rest = &args[values.byte_offset()..];
			}
			_ => rest = args,
		}
	}
}

enum Row {
	Data(Value),
	Module(String, Value),
	Other,
}

// Text rows (`<id>:T<hex>,<bytes>`) have no trailing newline before the next row.
fn parse_rows(payload: &str) -> (Vec<Value>, BTreeMap<String, Value>) {
	let mut rows = Vec::new();
	let mut modules = BTreeMap::new();
	let mut rest = payload;
	while !rest.is_empty() {
		match next_row(rest) {
			Some((row, after)) => {
				match row {
					Row::Data(value) => rows.push(value),
					Row::Module(id, value) => {
						modules.insert(id, value);
					}
					Row::Other => {}
				}
				rest = after;
			}
			None => rest = skip_line(rest),
		}
	}
	(rows, modules)
}

fn next_row(text: &str) -> Option<(Row, &str)> {
	let (id, body) = text.split_once(':')?;
	if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
		return None;
	}
	match body.chars().next()? {
		'T' => {
			let (length, content) = body[1..].split_once(',')?;
			let length = usize::from_str_radix(length, 16).ok()?;
			Some((Row::Other, content.get(length..)?))
		}
		'I' => {
			let (value, after) = json_value(&body[1..])?;
			Some((Row::Module(String::from(id), value), after))
		}
		tag if tag.is_ascii_uppercase() => Some((Row::Other, skip_line(body))),
		_ => {
			let (value, after) = json_value(body)?;
			Some((Row::Data(value), after))
		}
	}
}

fn json_value(text: &str) -> Option<(Value, &str)> {
	let mut stream = serde_json::Deserializer::from_str(text).into_iter::<Value>();
	let value = stream.next()?.ok()?;
	let after = &text[stream.byte_offset()..];
	Some((value, after.strip_prefix('\n').unwrap_or(after)))
}

fn skip_line(text: &str) -> &str {
	text.find('\n').map_or("", |at| &text[at + 1..])
}

fn search<'a>(value: &'a Value, keys: &[&str]) -> Option<&'a Map<String, Value>> {
	match value {
		Value::Object(map) => {
			if keys.iter().all(|k| map.contains_key(*k)) {
				return Some(map);
			}
			map.values().find_map(|v| search(v, keys))
		}
		Value::Array(items) => items.iter().find_map(|v| search(v, keys)),
		_ => None,
	}
}

fn collect<'a>(value: &'a Value, keys: &[&str], out: &mut Vec<&'a Map<String, Value>>) {
	match value {
		Value::Object(map) => {
			if keys.iter().all(|k| map.contains_key(*k)) {
				out.push(map);
			}
			for v in map.values() {
				collect(v, keys, out);
			}
		}
		Value::Array(items) => {
			for v in items {
				collect(v, keys, out);
			}
		}
		_ => {}
	}
}

fn collect_arrays<'a>(value: &'a Value, out: &mut Vec<&'a Vec<Value>>) {
	match value {
		Value::Array(items) => {
			if !items.is_empty() && items.iter().all(Value::is_object) {
				out.push(items);
			}
			for v in items {
				collect_arrays(v, out);
			}
		}
		Value::Object(map) => {
			for v in map.values() {
				collect_arrays(v, out);
			}
		}
		_ => {}
	}
}

fn reference<'a>(value: &'a Value, target: &Map<String, Value>) -> Option<&'a str> {
	match value {
		Value::Array(items) => {
			if let [
				Value::String(marker),
				Value::String(ref_id),
				_,
				Value::Object(props),
				..,
			] = items.as_slice()
				&& marker == "$"
				&& let Some(id) = ref_id.strip_prefix("$L")
				&& core::ptr::eq(props, target)
			{
				return Some(id);
			}
			items.iter().find_map(|item| reference(item, target))
		}
		Value::Object(map) => map.values().find_map(|item| reference(item, target)),
		_ => None,
	}
}

pub struct Classified {
	pub slug: String,
	pub text: String,
}

pub fn slug_like(value: &str) -> bool {
	!value.is_empty()
		&& !value
			.chars()
			.any(|c| c.is_whitespace() || c.is_uppercase() || c == '/')
}

impl Classified {
	// Next.js builds rename slug and title keys to short tokens while leaving `known` intact.
	pub fn of(records: &[&Map<String, Value>], known: &[&str]) -> Option<Self> {
		let first = records.first()?;
		if first.contains_key("series_slug") && first.contains_key("title") {
			return Some(Self {
				slug: "series_slug".into(),
				text: "title".into(),
			});
		}
		let mut strings = Vec::new();
		for key in first.keys().filter(|k| !known.contains(&k.as_str())) {
			if records
				.iter()
				.all(|record| matches!(record.get(key), Some(Value::String(_))))
			{
				strings.push(key.clone());
			}
		}
		if strings.len() < 2 {
			return None;
		}
		let slug_score = |key: &String| {
			records
				.iter()
				.filter(|record| {
					record
						.get(key)
						.and_then(Value::as_str)
						.is_some_and(slug_like)
				})
				.count()
		};
		let slug = strings
			.iter()
			.max_by_key(|k| slug_score(k))
			.filter(|k| slug_score(k) * 4 >= records.len() * 3)?
			.clone();
		let text = strings
			.iter()
			.find(|k| {
				**k != slug
					&& records.iter().all(|r| {
						r.get(*k).and_then(Value::as_str).is_some_and(|s| {
							!s.starts_with("http://") && !s.starts_with("https://")
						})
					})
			})?
			.clone();
		Some(Self { slug, text })
	}
}
