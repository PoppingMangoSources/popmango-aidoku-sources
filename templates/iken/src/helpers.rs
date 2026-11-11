use crate::{
	Params,
	models::{ChaptersResponse, IkenChapter, Post, PostResponse},
};
use aidoku::{
	Result,
	alloc::{String, Vec},
	helpers::uri::{QueryParameters, decode_uri},
	imports::net::{Request, Response},
	prelude::*,
};
use serde::de::DeserializeOwned;

pub fn check_response(response: Response, url: &str) -> Result<Response> {
	let status = response.status_code();
	if !(200..300).contains(&status) {
		let content_type = response.get_header("Content-Type");
		let prefix = response
			.get_string()
			.ok()
			.map(|s| s.chars().take(240).collect::<String>());
		println!(
			"URL: {url}; HTTP {status}; Content-Type: {content_type:?}; body prefix: {prefix:?}"
		);
		bail!("HTTP {status}");
	}
	Ok(response)
}

fn response(params: &Params, path: &str) -> Result<Response> {
	Ok(Request::get(format!("{}{path}", params.api_url()))?
		.header("Accept", "application/json")
		.header("Referer", &format!("{}/", params.base_url))
		.send()?)
}

pub fn json<T: DeserializeOwned>(params: &Params, path: &str) -> Result<T> {
	check_response(
		response(params, path)?,
		&format!("{}{path}", params.api_url()),
	)?
	.get_json()
}

pub fn post(params: &Params, key: &str) -> Result<Option<Post>> {
	let mut qs = QueryParameters::new();
	if key.parse::<i32>().is_ok() {
		qs.push("postId", Some(key));
	} else {
		let slug = decode_uri(
			key.strip_prefix("/series/")
				.unwrap_or(key)
				.split(['?', '#'])
				.next()
				.unwrap_or(key)
				.trim_end_matches('/'),
		);
		qs.push("postSlug", Some(&slug));
	}
	let path = format!("/api/post?{qs}");
	let response = response(params, &path)?;
	if response.status_code() == 404 {
		return Ok(None);
	}
	Ok(Some(
		check_response(response, &format!("{}{path}", params.api_url()))?
			.get_json::<PostResponse>()?
			.post,
	))
}

pub fn chapters(params: &Params, id: &str) -> Result<Vec<IkenChapter>> {
	let mut chapters: Vec<IkenChapter> = Vec::new();
	loop {
		let data: ChaptersResponse = json(
			params,
			&format!("/api/chapters?postId={id}&skip={}&take=100", chapters.len()),
		)?;
		let page = data.post.chapters;
		if page.is_empty() && chapters.len() < data.total_chapter_count {
			bail!("Incomplete chapter list");
		}
		if page
			.first()
			.is_some_and(|first| chapters.iter().any(|c| c.id == first.id))
		{
			bail!("Repeating chapter page");
		}
		chapters.extend(page);
		if chapters.len() >= data.total_chapter_count {
			break;
		}
	}
	Ok(chapters)
}
