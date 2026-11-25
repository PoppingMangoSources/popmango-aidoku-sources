#![no_std]
mod helpers;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent, HomeComponentValue,
	HomeLayout, ImageRequestProvider, Link, Listing, ListingProvider, Manga, MangaPageResult,
	MangaStatus, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, collections::BTreeSet, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::net::Request,
	prelude::*,
};
use helpers::{
	BASE_URL, SORTS, abs_url, chapter_id_from_href, clean, content_rating_for, fetch,
	home_section_latest, home_section_manga, listing, parse_catalogue, parse_chapter_label,
	parse_iso_utc, parse_status, slug_from_href, viewer_for,
};

struct MangaCherri;

impl Source for MangaCherri {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		if query.is_some() {
			qs.push("q", query.as_deref());
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						qs.set("sort", Some(sort));
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => {
					qs.set(&id, Some(&value));
				}
				_ => {}
			}
		}
		if page > 1 {
			qs.set("page", Some(&page.to_string()));
		}
		let url = if qs.is_empty() {
			format!("{BASE_URL}/manga")
		} else {
			format!("{BASE_URL}/manga?{qs}")
		};
		Ok(parse_catalogue(&fetch(&url)?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{BASE_URL}/title/{}", manga.key);
		let document = fetch(&url)?;

		if needs_details {
			if let Some(title) = document
				.select_first(".cherri-detail h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = document
				.select_first(".cherri-detail__cover img")
				.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
				.filter(|s| !s.is_empty())
				.map(|s| abs_url(&s))
				.or_else(|| manga.cover.take());
			manga.description = document
				.select_first(".cherri-detail__description")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			manga.authors = document
				.select_first(
					".cherri-detail__copy > p > strong, .cherri-detail__description + p strong",
				)
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.map(|a| vec![a]);

			let mut genres: Vec<String> = document
				.select(".cherri-detail__chips a")
				.into_iter()
				.flatten()
				.filter_map(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.collect();
			genres.sort_unstable();
			genres.dedup();
			manga.content_rating = content_rating_for(&genres);
			manga.viewer = viewer_for(&genres);
			manga.tags = (!genres.is_empty()).then_some(genres);
			manga.status = document
				.select_first(".cherri-detail__copy > p.eyebrow, .cherri-detail p.eyebrow")
				.and_then(|el| el.text())
				.map(|t| parse_status(t.split('·').next_back().unwrap_or(&t)))
				.unwrap_or(MangaStatus::Unknown);
			manga.url = Some(url);
		}

		if needs_chapters {
			let mut seen = BTreeSet::new();
			let chapters = document
				.select("[data-chapter-list] a[data-chapter-row], a.cherri-chapter-row")
				.into_iter()
				.flatten()
				.filter_map(|row| {
					let id = chapter_id_from_href(&row.attr("href")?)?;
					if !seen.insert(id.clone()) {
						return None;
					}
					let raw_name = row
						.select_first("strong")
						.and_then(|el| el.own_text().or_else(|| el.text()))
						.map(|t| clean(&t))
						.unwrap_or_default();
					let (chapter_number, title) = parse_chapter_label(&raw_name);
					let date_uploaded = row
						.select_first("time")
						.and_then(|el| el.attr("datetime"))
						.and_then(|dt| parse_iso_utc(&dt));
					Some(Chapter {
						url: Some(format!("{BASE_URL}/read/{id}")),
						key: id,
						title,
						chapter_number,
						date_uploaded,
						..Default::default()
					})
				})
				.collect();
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/read/{}", chapter.key);
		let document = fetch(&url)?;
		let pages: Vec<Page> = document
			.select("[data-cherri-reader] img[data-page], #chapter-reader img[data-page]")
			.into_iter()
			.flatten()
			.filter_map(|img| {
				let src = img.attr("data-src").or_else(|| img.attr("src"))?;
				(!src.is_empty()).then(|| Page {
					content: PageContent::url(abs_url(&src)),
					..Default::default()
				})
			})
			.collect();
		if pages.is_empty() {
			bail!("No pages found");
		}
		Ok(pages)
	}
}

impl ListingProvider for MangaCherri {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let mut filters = Vec::new();
		let sort = match listing.id.as_str() {
			"unfolding" => {
				filters.push(FilterValue::Select {
					id: "status".into(),
					value: "ongoing".into(),
				});
				0
			}
			"completed" => {
				filters.push(FilterValue::Select {
					id: "status".into(),
					value: "completed".into(),
				});
				0
			}
			"added" => 1,
			"popular" => 2,
			_ => 0,
		};
		filters.push(FilterValue::Sort {
			id: "sort".into(),
			index: sort,
			ascending: false,
		});
		self.get_search_manga_list(None, page, filters)
	}
}

impl Home for MangaCherri {
	fn get_home(&self) -> Result<HomeLayout> {
		let home = fetch(&format!("{BASE_URL}/home"))?;
		let mut components = Vec::new();

		let latest = home_section_latest(&home, "New chapter blooms");
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Updates".into()),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: Some(listing("updated", "Latest Updates")),
				},
			});
		}

		let most_viewed: Vec<Link> = home_section_manga(&home, "Most viewed manga")
			.into_iter()
			.map(Into::into)
			.collect();
		if !most_viewed.is_empty() {
			components.push(HomeComponent {
				title: Some("Most Viewed".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: most_viewed,
					listing: Some(listing("popular", "Most Viewed")),
				},
			});
		}

		let unfolding: Vec<Link> = home_section_manga(&home, "Stories still unfolding")
			.into_iter()
			.map(Into::into)
			.collect();
		if !unfolding.is_empty() {
			components.push(HomeComponent {
				title: Some("Stories Still Unfolding".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: unfolding,
					listing: Some(listing("unfolding", "Stories Still Unfolding")),
				},
			});
		}

		let completed: Vec<Link> = home_section_manga(&home, "Happily ever after")
			.into_iter()
			.map(Into::into)
			.collect();
		if !completed.is_empty() {
			components.push(HomeComponent {
				title: Some("Completed".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: completed,
					listing: Some(listing("completed", "Completed")),
				},
			});
		}

		let recent: Vec<Link> = home_section_manga(&home, "First encounters")
			.into_iter()
			.map(Into::into)
			.collect();
		if !recent.is_empty() {
			components.push(HomeComponent {
				title: Some("Recently Added".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: recent,
					listing: Some(listing("added", "Recently Added")),
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

impl ImageRequestProvider for MangaCherri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaCherri {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		if let Some(key) = slug_from_href(path) {
			return Ok(Some(DeepLinkResult::Manga { key }));
		}
		Ok(None)
	}
}

register_source!(
	MangaCherri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
