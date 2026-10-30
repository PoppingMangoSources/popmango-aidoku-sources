#![no_std]
use aidoku::{
	FilterValue, MangaPageResult, Result, Source,
	alloc::{String, Vec, string::ToString},
	helpers::uri::QueryParameters,
	prelude::*,
};
use mangak::{Impl, Params};

#[cfg(test)]
mod test;

const BASE_URL: &str = "https://mangak.io";
const API_URL: &str = "https://api.mangak.io";

// Best Match sends no sort, as on the site
const SORTS: [Option<&str>; 11] = [
	None,
	Some("latest"),
	Some("newest"),
	Some("popular"),
	Some("rating"),
	Some("views_today"),
	Some("views_7days"),
	Some("views_30days"),
	Some("views"),
	Some("chapters"),
	Some("alphabetical"),
];

struct MangaK;

impl Impl for MangaK {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			api_url: API_URL.into(),
		}
	}

	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		if let Some(query) = query {
			qs.push("q", Some(&query));
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize).copied().flatten() {
						qs.push("sort", Some(sort));
					}
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} if id == "genres" => {
					qs.push("genres", Some(&included.join(",")));
					qs.push("exclude", Some(&excluded.join(",")));
				}
				FilterValue::MultiSelect { id, included, .. } => {
					qs.push(&id, Some(&included.join(",")));
				}
				// The API answers an empty status or author with a 400
				FilterValue::Select { id, value } | FilterValue::Text { id, value }
					if !value.is_empty() =>
				{
					qs.push(&id, Some(&value));
				}
				FilterValue::Range { from, to, .. } => {
					for (name, bound) in [("min_ch", from), ("max_ch", to)] {
						if let Some(bound) = bound {
							qs.push(name, Some(&(bound as i32).to_string()));
						}
					}
				}
				_ => {}
			}
		}
		self.search(params, qs, page)
	}
}

register_source!(
	mangak::MangaK<MangaK>,
	ListingProvider,
	Home,
	ImageRequestProvider,
	DeepLinkHandler
);
