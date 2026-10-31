use crate::{Params, helpers::*, models::*};
use aidoku::{
	Chapter, DeepLinkResult, FilterValue, HomeComponent, HomeComponentValue, HomeLayout,
	HomePartialResult, Link, Listing, Manga, MangaPageResult, Page, PageContent, PageContext,
	Result,
	alloc::{String, Vec, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::{net::Request, std::send_partial_result},
	prelude::*,
};

pub trait Impl {
	fn new() -> Self;

	fn params(&self) -> Params;

	fn search(
		&self,
		params: &Params,
		mut qs: QueryParameters,
		page: i32,
	) -> Result<MangaPageResult> {
		qs.push("page", Some(&page.to_string()));
		let response: ApiResponse<SearchData> =
			Request::get(format!("{}/titles/search?{qs}", params.api_url))?.json_owned()?;
		Ok(response.data.into_page(&params.base_url))
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
					for (name, genres) in [("genres", included), ("exclude", excluded)] {
						if !genres.is_empty() {
							qs.push(name, Some(&genres.join(",")));
						}
					}
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
				FilterValue::Check { id, value } if value != 0 => {
					qs.push(&id, Some("true"));
				}
				_ => {}
			}
		}
		self.search(params, qs, page)
	}

	fn get_manga_list(
		&self,
		params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		match listing.id.as_str() {
			id @ ("trending" | "hot") => {
				let window = if id == "hot" { "?window=week" } else { "" };
				let response: ApiResponse<SearchData> =
					Request::get(format!("{}/trending/titles{window}", params.api_url))?
						.json_owned()?;
				return Ok(response.data.into_page(&params.base_url));
			}
			"popular_updates" => {
				qs.push("sort", Some("bookmarks"));
				qs.push("window", Some("today"));
			}
			"new" => qs.push("sort", Some("newest")),
			id @ ("uncensored" | "raw") => qs.push(&format!("is_{id}"), Some("true")),
			_ => qs.push("sort", Some("latest")),
		}
		self.search(params, qs, page)
	}

	fn get_manga_update(
		&self,
		params: &Params,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let page: MangaPage = next_data(&format!("{}{}", params.base_url, manga.key))?;
		let details = page.initial_manga;
		let id = details.id.clone();

		if needs_details {
			manga.copy_from(details.into_manga(&params.base_url, manga.key.clone()));
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let response: ApiResponse<ChapterList> =
				Request::get(format!("{}/titles/{id}/chapters", params.api_url))?.json_owned()?;
			manga.chapters = Some(
				response
					.data
					.chapters
					.into_iter()
					.map(|chapter| chapter.into_chapter(&params.base_url))
					.collect(),
			);
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let page: ChapterPage = next_data(&format!("{}{}", params.base_url, chapter.key))?;
		Ok(page
			.initial_chapter
			.images
			.into_iter()
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect())
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		let mut sections = vec![(
			params.featured_title.as_ref(),
			params.featured_path.as_ref(),
			None,
		)];
		if params.trending_sections {
			sections.extend([
				("Hot this week", "trending/titles?window=week", Some("hot")),
				("Trending Today", "trending/titles", Some("trending")),
			]);
		}
		sections.extend([
			(
				"Popular Updates",
				"titles/search?sort=bookmarks&window=today&limit=12",
				Some("popular_updates"),
			),
			(
				"New Manga",
				"titles/search?sort=newest&limit=18",
				Some("new"),
			),
			(
				params.latest_title.as_ref(),
				"titles/search?sort=latest",
				Some("latest"),
			),
		]);
		send_partial_result(&HomePartialResult::Layout(HomeLayout {
			components: sections
				.iter()
				.map(|(title, _, listing)| {
					home_component(&params.base_url, title, *listing, Vec::new())
				})
				.collect(),
		}));

		let requests = sections
			.iter()
			.map(|(_, path, _)| Ok(Request::get(format!("{}/{path}", params.api_url))?))
			.collect::<Result<Vec<_>>>()?;
		// Returning the final layout drops the placeholder of a section that failed
		let components = sections
			.iter()
			.zip(Request::send_all(requests))
			.filter_map(|((title, _, listing), response)| {
				let response = response
					.ok()?
					.get_json_owned::<ApiResponse<SearchData>>()
					.ok()?;
				Some(home_component(
					&params.base_url,
					title,
					*listing,
					response.data.items,
				))
			})
			.collect::<Vec<_>>();
		if components.is_empty() {
			bail!("The site didn't answer");
		}
		Ok(HomeLayout { components })
	}

	fn get_image_request(
		&self,
		params: &Params,
		url: String,
		_context: Option<PageContext>,
	) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{}/", params.base_url)))
	}

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(params.base_url.as_ref()) else {
			return Ok(None);
		};
		let parts: Vec<&str> = path.split('/').filter(|part| !part.is_empty()).collect();
		Ok(match parts[..] {
			[manga] => Some(DeepLinkResult::Manga {
				key: format!("/{manga}"),
			}),
			[manga, chapter] => Some(DeepLinkResult::Chapter {
				manga_key: format!("/{manga}"),
				key: format!("/{manga}/{chapter}"),
			}),
			_ => None,
		})
	}
}

fn home_component(
	base_url: &str,
	title: &str,
	listing_id: Option<&str>,
	items: Vec<SearchItem>,
) -> HomeComponent {
	let link = listing_id.map(|id| Listing {
		id: id.into(),
		name: title.into(),
		..Default::default()
	});
	let value = match listing_id {
		None => HomeComponentValue::BigScroller {
			entries: items
				.into_iter()
				.map(|item| item.into_featured(base_url))
				.collect(),
			auto_scroll_interval: None,
		},
		// The site shows each title's newest chapter in these rows
		Some("popular_updates" | "latest") => HomeComponentValue::MangaChapterList {
			page_size: Some(5),
			entries: items
				.into_iter()
				.filter_map(|item| item.into_manga_with_chapter(base_url))
				.collect(),
			listing: link,
		},
		Some(_) => HomeComponentValue::Scroller {
			entries: items
				.into_iter()
				.map(|item| item.into_manga(base_url).into())
				.collect::<Vec<Link>>(),
			listing: link,
		},
	};
	HomeComponent {
		title: Some(title.into()),
		subtitle: None,
		value,
	}
}
