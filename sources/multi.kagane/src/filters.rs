use aidoku::{
	DynamicFilters, DynamicSettings, Filter, GroupSetting, MultiSelectFilter, MultiSelectSetting,
	Result, Setting,
	alloc::{String, Vec, format, vec},
	helpers::uri::encode_uri_component,
	imports::net::Request,
	prelude::*,
};
use serde_json::json;

use crate::{
	Kagane,
	helpers::{api_url, get, parse, post, sources_request},
	models::{GenreEntry, GroupResults, SourceList, StaffResults, TagEntry},
	settings,
};

fn genres_request(base_url: &str) -> Result<Request> {
	get(base_url, &api_url(base_url, "/genres/list"))
}

pub fn genres(base_url: &str) -> Result<Vec<GenreEntry>> {
	parse(genres_request(base_url)?.send()?)
}

pub fn tags(base_url: &str) -> Result<Vec<TagEntry>> {
	parse(get(base_url, &api_url(base_url, "/tags/list"))?.send()?)
}

/// Every id listed under `name`, since the site lists some names more than once.
pub fn tag_ids(tags: &[TagEntry], name: &str) -> Vec<String> {
	tags.iter()
		.filter(|tag| tag.tag_name.eq_ignore_ascii_case(name))
		.map(|tag| tag.id.clone())
		.collect()
}

/// Ids of the hidden tags setting, skipping names the site no longer lists.
pub fn hidden_tag_ids(base_url: &str) -> Result<Vec<String>> {
	if settings::hidden_tags().is_empty() {
		return Ok(Vec::new());
	}
	Ok(hidden_ids(&tags(base_url)?))
}

pub fn hidden_ids(tags: &[TagEntry]) -> Vec<String> {
	settings::hidden_tags()
		.iter()
		.flat_map(|name| tag_ids(tags, name.trim()))
		.collect()
}

/// Comma-separated tag names, a leading `-` excluding one, as included and excluded ids.
pub fn typed_tags(tags: &[TagEntry], input: &str) -> Result<(Vec<String>, Vec<String>)> {
	let mut included = Vec::new();
	let mut excluded = Vec::new();
	for entry in input
		.split(',')
		.map(str::trim)
		.filter(|entry| !entry.is_empty())
	{
		let (list, name) = match entry.strip_prefix('-') {
			Some(name) => (&mut excluded, name.trim()),
			None => (&mut included, entry),
		};
		let ids = tag_ids(tags, name);
		if ids.is_empty() {
			bail!("Kagane has no tag named {name}");
		}
		list.extend(ids);
	}
	Ok((included, excluded))
}

/// The one candidate matching `name`, preferring exact matches.
fn single_match(candidates: Vec<(String, String)>, name: &str) -> Result<String> {
	let exact: Vec<_> = candidates
		.iter()
		.filter(|(_, candidate)| candidate.eq_ignore_ascii_case(name))
		.collect();
	match (exact.as_slice(), candidates.as_slice()) {
		([(id, _)], _) | ([], [(id, _)]) => Ok(id.clone()),
		([], []) => bail!("Kagane has no matches for {name}"),
		_ => bail!("Kagane has several matches for {name}; enter the full name"),
	}
}

pub fn staff_id(base_url: &str, name: &str) -> Result<String> {
	let results: StaffResults = parse(
		post(
			base_url,
			&api_url(base_url, "/search/staff?size=8"),
			&json!({ "name": name }),
		)?
		.send()?,
	)?;
	single_match(
		results
			.content
			.into_iter()
			.map(|staff| (staff.staff_id, staff.name))
			.collect(),
		name,
	)
}

pub fn group_id(base_url: &str, name: &str) -> Result<String> {
	let url = api_url(
		base_url,
		&format!("/groups/search?q={}&limit=8", encode_uri_component(name)),
	);
	let results: GroupResults = parse(get(base_url, &url)?.send()?)?;
	single_match(
		results
			.items
			.into_iter()
			.map(|group| (group.group_id, group.title))
			.collect(),
		name,
	)
}

impl DynamicFilters for Kagane {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let base_url = settings::base_url();
		let mut responses = Request::send_all(vec![
			genres_request(&base_url)?,
			sources_request(&base_url)?,
		])
		.into_iter();
		let mut next = || {
			responses
				.next()
				.ok_or_else(|| error!("Kagane sent no response"))
		};
		let mut genres: Vec<GenreEntry> = parse(next()??)?;
		genres.sort_by(|a, b| a.genre_name.cmp(&b.genre_name));
		let mut sources = parse::<SourceList>(next()??)?.sources;
		sources.sort_by(|a, b| a.title.cmp(&b.title));

		let mut filters = Vec::new();
		if !genres.is_empty() {
			let (ids, options) = genres
				.into_iter()
				.map(|genre| (genre.id.into(), genre.genre_name.into()))
				.unzip();
			filters.push(
				MultiSelectFilter {
					id: "genres".into(),
					title: Some("Genres".into()),
					is_genre: true,
					can_exclude: true,
					uses_tag_style: true,
					options,
					ids: Some(ids),
					..Default::default()
				}
				.into(),
			);
		}
		if !sources.is_empty() {
			let (ids, options) = sources
				.into_iter()
				.map(|source| (source.source_id.into(), source.title.into()))
				.unzip();
			filters.push(
				MultiSelectFilter {
					id: "sources".into(),
					title: Some("Sources".into()),
					options,
					ids: Some(ids),
					..Default::default()
				}
				.into(),
			);
		}
		Ok(filters)
	}
}

/// The genre list lives on the site, so hidden genres are offered from it.
impl DynamicSettings for Kagane {
	fn get_dynamic_settings(&self) -> Result<Vec<Setting>> {
		let mut genres = genres(&settings::base_url())?;
		genres.sort_by(|a, b| a.genre_name.cmp(&b.genre_name));
		let (values, titles) = genres
			.into_iter()
			.map(|genre| (genre.id.into(), genre.genre_name.into()))
			.unzip();
		Ok(vec![
			GroupSetting {
				title: "Hidden Genres".into(),
				footer: Some(
					"Titles in any of these are left out of Discover, listings and browsing, but \
					 not out of a title search or a search that picks the genre."
						.into(),
				),
				items: vec![
					MultiSelectSetting {
						key: "hiddenGenres".into(),
						title: "Genres".into(),
						values,
						titles: Some(titles),
						refreshes: Some(vec!["content".into(), "listings".into()]),
						..Default::default()
					}
					.into(),
				],
				..Default::default()
			}
			.into(),
		])
	}
}
