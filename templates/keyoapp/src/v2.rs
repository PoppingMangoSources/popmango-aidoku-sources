use crate::{
	Params,
	helpers::{
		card_manga, chapter_date, chapter_parts, content_rating, labelled_block, names,
		recent_chapter, series_card, status, viewer,
	},
	models::{ChaptersResponse, SearchResponse},
};
use aidoku::{
	Chapter, DeepLinkResult, FilterValue, HomeComponent, HomeComponentValue, HomeLayout, Link,
	Manga, MangaPageResult, MangaWithChapter, Page, PageContent, Result,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{
		string::{PlainText, StripPrefixOrSelf},
		uri::QueryParameters,
	},
	imports::{
		html::{Document, Html},
		net::Request,
		std::send_partial_result,
	},
	prelude::*,
};

const SORTS: [&str; 7] = [
	"latest", "trending", "popular", "views", "rating", "az", "za",
];

pub trait ImplV2 {
	fn new() -> Self;

	fn params(&self) -> Params;

	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		// A pasted series link finds that series
		if let Some(path) = query
			.as_deref()
			.and_then(|query| query.strip_prefix(params.base_url.as_ref()))
		{
			let html = Request::get(format!("{}{path}", params.base_url))?.html()?;
			if let Some(manga) = series_card(&html) {
				return Ok(MangaPageResult {
					entries: vec![manga],
					has_next_page: false,
				});
			}
		}

		let mut qs = QueryParameters::new();
		qs.push("_ajax", Some("search"));
		if let Some(query) = query {
			qs.push("q", Some(&query));
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					// The site's default sort sends nothing
					if let Some(sort) = SORTS.get(index as usize).filter(|_| index > 0) {
						qs.push("sort", Some(sort));
					}
				}
				// A tapped tag sends its name, which the site matches lowercased
				FilterValue::Select { id, value } if id == "genre" => {
					qs.push("tag[]", Some(&value.to_lowercase()));
				}
				FilterValue::Select { id, value } if !value.is_empty() => {
					qs.push(&id, Some(&value));
				}
				FilterValue::MultiSelect { included, .. } => {
					for tag in included {
						qs.push("tag[]", Some(&tag));
					}
				}
				_ => {}
			}
		}
		qs.push("offset", Some(&((page - 1) * 20).to_string()));
		let response: SearchResponse =
			Request::get(format!("{}/?{qs}", params.base_url))?.json_owned()?;
		let html = Html::parse_fragment_with_url(response.html, params.base_url.as_ref())?;
		Ok(MangaPageResult {
			entries: html
				.select("a.group[href]")
				.map(|cards| {
					cards
						.filter_map(|card| card_manga(&card, &params.base_url))
						.collect()
				})
				.unwrap_or_default(),
			has_next_page: response.has_more,
		})
	}

	fn get_manga_update(
		&self,
		params: &Params,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let slug = manga.key.trim_start_matches('/');
		if needs_details {
			let url = format!("{}{}", params.base_url, manga.key);
			let html = Request::get(&url)?.html()?;
			// An unknown slug still answers with a page, one without a series
			let Some(card) = series_card(&html) else {
				bail!("Series not found");
			};
			manga.title = card.title;
			manga.cover = card.cover.or(manga.cover);
			manga.description = html
				.select_first("#series-desc")
				.and_then(|description| description.text())
				.map(|description| description.trim().escape_markdown())
				.filter(|description| !description.is_empty());
			manga.authors = html
				.select_first("p.l2-by")
				.and_then(|by| by.text())
				.map(|by| names(by.trim().trim_start_matches("By ")))
				.or_else(|| labelled_block(&html, "Author").map(|author| names(&author)));
			manga.artists = labelled_block(&html, "Artist").map(|artist| names(&artist));
			// "Mangatoon · Action · Ongoing": the type, a genre, then the status
			let meta = html
				.select_first(".l2-meta span")
				.and_then(|meta| meta.text());
			let mut parts = meta.iter().flat_map(|meta| meta.split(" · "));
			let kind = parts
				.next()
				.map(String::from)
				.or_else(|| labelled_block(&html, "Type"));
			let state = parts
				.last()
				.map(String::from)
				.or_else(|| labelled_block(&html, "Status"));
			manga.viewer = kind.map(|kind| viewer(&kind)).unwrap_or_default();
			manga.status = state.map(|state| status(&state)).unwrap_or_default();
			let tags: Vec<String> = html
				.select(".l2-tagline a")
				.filter(|links| !links.is_empty())
				.or_else(|| html.select("a[href*='tag%5B%5D']"))
				.map(|links| {
					links
						.filter_map(|link| link.text())
						.map(|tag| tag.trim().into())
						.collect()
				})
				.unwrap_or_default();
			manga.content_rating = content_rating(tags.iter().map(String::as_str));
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url);
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			let rows: ChaptersResponse = Request::get(format!(
				"{}/?_chapters_html=1&series_slug={slug}",
				params.base_url
			))?
			.json_owned()?;
			let html = Html::parse_fragment_with_url(rows.rows_html, params.base_url.as_ref())?;
			manga.chapters = html.select("a.chapter-row[href]").map(|rows| {
				rows
					// Coin chapters need a login
					.filter(|row| row.attr("data-ch-locked").as_deref() != Some("1"))
					.filter_map(|row| {
						let key: String = row.attr("href")?.strip_prefix_or_self(&params.base_url).into();
						let (chapter_number, title) = chapter_parts(&row.attr("data-ch-label")?);
						Some(Chapter {
							url: Some(format!("{}{key}", params.base_url)),
							key,
							title,
							chapter_number,
							date_uploaded: row
								.select_first(".ch-date-row span")
								.and_then(|date| date.text())
								.and_then(|date| chapter_date(&date)),
							..Default::default()
						})
					})
					.collect()
			});
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let html = Request::get(format!("{}{}", params.base_url, chapter.key))?.html()?;
		let pages = html
			.select("#strip-reader img.reader-page")
			.map(|images| {
				images
					.map(|image| {
						let url = image
							.attr("src")
							.ok_or_else(|| error!("A page has no image"))?;
						Ok(Page {
							content: PageContent::url(url),
							..Default::default()
						})
					})
					.collect::<Result<Vec<Page>>>()
			})
			.transpose()?
			.unwrap_or_default();
		if pages.is_empty() {
			bail!("No pages found");
		}
		Ok(pages)
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		let html = Request::get(&params.base_url)?.html()?;
		Ok(self.parse_home(params, &html))
	}

	// Apart from the request, so tests can check a saved page
	fn parse_home(&self, params: &Params, html: &Document) -> HomeLayout {
		let mut components: Vec<HomeComponent> = Vec::new();
		for section in html.select("section").into_iter().flatten() {
			let Some(title) = section
				.select_first("h2")
				.and_then(|heading| heading.text())
				.map(|title| title.trim().to_string())
			else {
				continue;
			};
			// Editor's Choice repeats for the mobile layout
			if components
				.iter()
				.any(|component| component.title.as_ref() == Some(&title))
				|| params.hidden_rows.contains(&title.as_str())
			{
				continue;
			}
			let value = if section.select_first("article").is_some() {
				let entries: Vec<MangaWithChapter> = section
					.select("article")
					.map(|cards| {
						cards
							.filter_map(|card| recent_chapter(&card, &params.base_url))
							.collect()
					})
					.unwrap_or_default();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries,
					listing: None,
				}
			} else {
				let entries: Vec<Link> = section
					.select("a[data-slide-card], a[data-ec-card], a.group[href]")
					.map(|cards| {
						cards
							.filter_map(|card| card_manga(&card, &params.base_url))
							.map(Link::from)
							.collect()
					})
					.unwrap_or_default();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::Scroller {
					entries,
					listing: None,
				}
			};
			components.push(HomeComponent {
				title: Some(title),
				subtitle: None,
				value,
			});
		}
		HomeLayout { components }
	}

	// Series and chapter pages share one path shape, so the page says which it is
	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(params.base_url.as_ref()) else {
			return Ok(None);
		};
		if path.trim_matches('/').is_empty() || path.contains('?') {
			return Ok(None);
		}
		let html = Request::get(&url)?.html()?;
		if html.select_first("[data-series-uid]").is_some() {
			return Ok(Some(DeepLinkResult::Manga { key: path.into() }));
		}
		Ok(html
			.select_first("#sidebar-ch-list")
			.and_then(|list| list.attr("data-series-slug"))
			.map(|slug| DeepLinkResult::Chapter {
				manga_key: format!("/{slug}"),
				key: path.into(),
			}))
	}
}
