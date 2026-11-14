use crate::models::{API_URL, BASE_URL, GraphQLResponse};
use aidoku::{
	Result,
	alloc::Vec,
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;
use serde_json::Value;

pub fn request(query: &str, variables: Value) -> Result<Request> {
	let mut payload = serde_json::Map::new();
	payload.insert("query".into(), query.into());
	payload.insert("variables".into(), variables);
	let body = serde_json::to_vec(&payload).map_err(|_| error!("Failed to build request body"))?;
	Ok(Request::post(API_URL)?
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.header("Referer", &format!("{BASE_URL}/"))
		.header("Origin", BASE_URL)
		.body(body))
}

// The API answers a rejected query with a 400 that still carries GraphQL errors
pub fn parse<T: DeserializeOwned>(response: Response) -> Result<T> {
	let parsed: GraphQLResponse<T> = response.get_json_owned()?;
	if let Some(errors) = parsed.errors.filter(|errors| !errors.is_empty()) {
		let message = errors
			.iter()
			.map(|error| error.message.as_str())
			.collect::<Vec<_>>()
			.join("\n");
		bail!("Mkissa returned an error: {message}");
	}
	parsed.data.ok_or(error!("Missing response data"))
}

pub fn fetch<T: DeserializeOwned>(query: &str, variables: Value) -> Result<T> {
	parse(request(query, variables)?.send()?)
}
