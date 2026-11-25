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
	imports::{net::Request, std::parse_date_with_options},
	prelude::*,
};
use helpers::{
	BASE_URL, SORTS, abs_url, chapter_from_href, clean, content_rating_for, fetch, has_next_page,
	listing, parse_cards_in, parse_chapter_label, parse_hero, parse_status, slug_from_href,
	viewer_for,
};

struct MangaBerri;

impl Source for MangaBerri {
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
			format!("{BASE_URL}/browse")
		} else {
			format!("{BASE_URL}/browse?{qs}")
		};
		let document = fetch(&url)?;
		Ok(MangaPageResult {
			entries: parse_cards_in(&document, "[data-browse-results] article.manga-card"),
			has_next_page: has_next_page(&document),
		})
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
				.select_first("section.detail h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = document
				.select_first("section.detail img.detail-cover")
				.and_then(|img| img.attr("data-src").or_else(|| img.attr("src")))
				.filter(|s| !s.is_empty())
				.map(|s| abs_url(&s))
				.or_else(|| manga.cover.take());
			manga.description = document
				.select_first("section.detail p.synopsis")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());
			manga.authors = document
				.select_first("section.detail h1 + p.muted")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.map(|a| vec![a]);

			let mut genres: Vec<String> = document
				.select("section.detail .tags a")
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
				.select_first("section.detail .eyebrow")
				.and_then(|el| el.text())
				.map(|t| parse_status(t.split('·').next().unwrap_or(&t)))
				.unwrap_or(MangaStatus::Unknown);
			manga.url = Some(url);
		}

		if needs_chapters {
			let mut seen = BTreeSet::new();
			let chapters = document
				.select(".chapter-list a[data-chapter-group]")
				.into_iter()
				.flatten()
				.filter_map(|row| {
					let (slug, id) = chapter_from_href(&row.attr("href")?)?;
					if !seen.insert(id.clone()) {
						return None;
					}
					let raw_name = row
						.select_first("strong")
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.unwrap_or_default();
					let (chapter_number, title) = parse_chapter_label(&raw_name);
					let date_uploaded =
						row.select_first("span")
							.and_then(|el| el.text())
							.and_then(|t| {
								parse_date_with_options(
									clean(&t),
									"MMM d, yyyy",
									"en_US_POSIX",
									"UTC",
								)
							});
					let scanlators = (row.attr("data-chapter-group").as_deref() == Some("raw"))
						.then(|| vec!["RAW".into()]);
					Some(Chapter {
						url: Some(format!("{BASE_URL}/read/{slug}/{id}")),
						key: id,
						title,
						chapter_number,
						date_uploaded,
						scanlators,
						..Default::default()
					})
				})
				.collect();
			manga.chapters = Some(chapters);
		}

		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{BASE_URL}/read/{}/{}", manga.key, chapter.key);
		let document = fetch(&url)?;
		let pages: Vec<Page> = document
			.select("img[data-page-image]")
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

impl ListingProvider for MangaBerri {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let index = match listing.id.as_str() {
			"popular" => 1,
			"title" => 2,
			_ => 0,
		};
		self.get_search_manga_list(
			None,
			page,
			vec![FilterValue::Sort {
				id: "sort".into(),
				index,
				ascending: false,
			}],
		)
	}
}

impl Home for MangaBerri {
	fn get_home(&self) -> Result<HomeLayout> {
		let doc = fetch(&format!("{BASE_URL}/discover"))?;
		let mut components = Vec::new();

		let featured = parse_hero(&doc);
		if !featured.is_empty() {
			components.push(HomeComponent {
				title: Some("Featured".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: featured,
					auto_scroll_interval: Some(6.0),
				},
			});
		}

		let popular: Vec<Link> = parse_cards_in(&doc, "section.popular-manga article.manga-card")
			.into_iter()
			.map(Into::into)
			.collect();
		if !popular.is_empty() {
			components.push(HomeComponent {
				title: Some("Popular".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: popular,
					listing: Some(listing("popular", "Popular")),
				},
			});
		}

		let latest: Vec<Link> =
			parse_cards_in(&doc, "section:not(.popular-manga) article.manga-card")
				.into_iter()
				.map(Into::into)
				.collect();
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Updates".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: latest,
					listing: Some(listing("latest", "Latest Updates")),
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

impl ImageRequestProvider for MangaBerri {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for MangaBerri {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		if let Some((manga_key, key)) = chapter_from_href(path) {
			return Ok(Some(DeepLinkResult::Chapter { manga_key, key }));
		}
		if let Some(key) = slug_from_href(path) {
			return Ok(Some(DeepLinkResult::Manga { key }));
		}
		Ok(None)
	}
}

register_source!(
	MangaBerri,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
