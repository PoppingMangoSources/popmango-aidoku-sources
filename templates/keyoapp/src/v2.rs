use crate::{
	Params,
	helpers::{chapter_date, chapter_parts, content_rating, is_novel, status, viewer},
	models::{ChapterRows, SearchResponse},
};
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent, HomeComponentValue,
	HomeLayout, Link, Manga, MangaPageResult, MangaWithChapter, Page, PageContent, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{
		string::{PlainText, StripPrefixOrSelf},
		uri::QueryParameters,
	},
	imports::{
		html::{Document, Element, Html},
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
			if let Some(manga) = html
				.select_first("[data-series-slug]")
				.and_then(|series| series.attr("data-series-slug"))
				.and_then(|slug| {
					Some(Manga {
						key: format!("/{slug}"),
						title: html.select_first("h1.l2-title")?.text()?,
						cover: html
							.select_first(".l2-poster img")
							.and_then(|image| image.attr("data-src")),
						..Default::default()
					})
				}) {
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
						.filter(|card| !is_novel(card))
						.filter_map(|card| {
							Some(Manga {
								key: card
									.attr("href")?
									.strip_prefix_or_self(&params.base_url)
									.into(),
								title: card.select_first("h2")?.text()?.trim().into(),
								cover: card
									.select_first("img")
									.and_then(|image| image.attr("data-src")),
								..Default::default()
							})
						})
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
			if html.select_first("[data-series-uid]").is_none() {
				bail!("Series not found");
			}
			// Sites choose between two series layouts: a banner one with `l2-` classes, and one of
			// labelled blocks
			if let Some(title) = html
				.select_first("h1.l2-title")
				.or_else(|| html.select_first("h1"))
				.and_then(|h1| h1.text())
			{
				manga.title = title.trim().into();
			}
			manga.cover = html
				.select_first(".l2-poster img")
				.and_then(|image| image.attr("data-src"))
				.or_else(|| {
					html.select_first("meta[property='og:image']")?
						.attr("content")
				})
				.or(manga.cover);
			manga.description = html
				.select_first("#series-desc")
				.and_then(|description| description.text())
				.map(|description| description.trim().escape_markdown())
				.filter(|description| !description.is_empty());
			let names = |text: String| -> Vec<String> {
				text.trim_start_matches("By ")
					.split(", ")
					.map(|name| name.trim().into())
					.collect()
			};
			manga.authors = html
				.select_first("p.l2-by")
				.and_then(|by| by.text())
				.or_else(|| block_value(&html, "Author"))
				.map(names);
			manga.artists = block_value(&html, "Artist").map(names);
			// "Mangatoon · Action · Ongoing": the type, a genre, then the status
			let meta = html
				.select_first(".l2-meta span")
				.and_then(|meta| meta.text())
				.unwrap_or_default();
			let parts: Vec<&str> = meta.split(" · ").filter(|part| !part.is_empty()).collect();
			let kind = parts
				.first()
				.map(|kind| kind.to_string())
				.or_else(|| block_value(&html, "Type"));
			let state = (parts.len() > 1)
				.then(|| parts.last().map(|state| state.to_string()))
				.flatten()
				.or_else(|| block_value(&html, "Status"));
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
			let rows: ChapterRows = Request::get(format!(
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

	// Each row is a section under its `h2`. Kept apart from the request so a layout can be checked
	// against a saved page
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
			// Editor's Choice repeats for the mobile layout, and the novels row, which the template
			// can't read, lists some novels untitled as such
			if components
				.iter()
				.any(|component| component.title.as_ref() == Some(&title))
				|| params.hidden_rows.contains(&title.as_str())
				|| section.select_first("#novel-slider").is_some()
			{
				continue;
			}
			let value = if section.select_first("article").is_some() {
				let entries: Vec<MangaWithChapter> = section
					.select("article")
					.map(|cards| {
						cards
							.filter_map(|card| newest_chapter(&card, &params.base_url))
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

// A labelled block's value: "Status" over "Ongoing", or "Author" beside the names
fn block_value(html: &Document, label: &str) -> Option<String> {
	html.select("div.bg-surface.rounded-lg")?.find_map(|block| {
		let name = block.select_first(".text-xs")?.text()?;
		if name.trim() != label {
			return None;
		}
		block
			.select_first("span.text-sm")?
			.text()
			.map(|value| value.trim().to_string())
			.filter(|value| !value.is_empty())
	})
}

// A home card: posters carry their title in `title`, newly added and image slider cards in a
// heading; novels, which the template can't read, are titled "[ Novel ] …"
fn card_manga(card: &Element, base_url: &str) -> Option<Manga> {
	let title = card
		.attr("title")
		.or_else(|| card.select_first("h2, h3")?.text())?
		.trim()
		.to_string();
	if title.starts_with("[ Novel ]") || is_novel(card) {
		return None;
	}
	Some(Manga {
		key: card.attr("href")?.strip_prefix_or_self(base_url).into(),
		title,
		cover: card
			.select_first("img")
			.and_then(|image| image.attr("data-src")),
		..Default::default()
	})
}

// A recently updated card lists its newest chapters, and coin ones need a login
fn newest_chapter(card: &Element, base_url: &str) -> Option<MangaWithChapter> {
	let series = card.select_first("h2 a")?;
	let link = card.select("a")?.find(|link| {
		link.select_first("time").is_some() && link.select_first("img[alt$=coins]").is_none()
	})?;
	let key: String = link.attr("href")?.strip_prefix_or_self(base_url).into();
	let (chapter_number, title) = chapter_parts(&link.select_first("span")?.text()?);
	Some(MangaWithChapter {
		manga: Manga {
			key: series.attr("href")?.strip_prefix_or_self(base_url).into(),
			title: series.text()?.trim().into(),
			cover: card
				.select_first("img")
				.and_then(|image| image.attr("data-src")),
			..Default::default()
		},
		chapter: Chapter {
			url: Some(format!("{base_url}{key}")),
			key,
			title,
			chapter_number,
			date_uploaded: link
				.select_first("time")
				.and_then(|time| time.text())
				.and_then(|date| chapter_date(&date)),
			..Default::default()
		},
	})
}

pub struct KeyoappV2<T: ImplV2> {
	inner: T,
	params: Params,
}

impl<T: ImplV2> Source for KeyoappV2<T> {
	fn new() -> Self {
		let inner = T::new();
		let params = inner.params();
		Self { inner, params }
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		self.inner
			.get_search_manga_list(&self.params, query, page, filters)
	}

	fn get_manga_update(
		&self,
		manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		self.inner
			.get_manga_update(&self.params, manga, needs_details, needs_chapters)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		self.inner.get_page_list(&self.params, manga, chapter)
	}
}

impl<T: ImplV2> Home for KeyoappV2<T> {
	fn get_home(&self) -> Result<HomeLayout> {
		self.inner.get_home(&self.params)
	}
}

impl<T: ImplV2> DeepLinkHandler for KeyoappV2<T> {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		self.inner.handle_deep_link(&self.params, url)
	}
}
