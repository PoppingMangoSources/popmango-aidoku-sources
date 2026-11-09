use crate::{
	Params,
	helpers::{
		background_image, chapter, content_rating, editor_pick, labelled, manga, names,
		newest_chapter, novel_text, section, status, viewer,
	},
};
use aidoku::{
	Chapter, DeepLinkResult, FilterValue, HomeComponent, HomeComponentValue, HomeLayout, Link,
	LinkValue, Manga, MangaPageResult, MangaWithChapter, Page, PageContent, Result,
	alloc::{String, Vec, vec},
	helpers::string::{PlainText, StripPrefixOrSelf},
	imports::{
		html::{Document, Html},
		net::Request,
	},
	prelude::*,
};

pub trait Impl {
	fn new() -> Self;

	fn params(&self) -> Params;

	// The site sends its whole catalogue on one page and filters it in the browser, ignoring query
	// parameters, so search does the same
	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		_page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut genres = Vec::new();
		let mut types = Vec::new();
		let mut statuses = Vec::new();
		for filter in filters {
			if let FilterValue::MultiSelect { id, included, .. } = filter {
				match id.as_str() {
					"genre" => genres = included.iter().map(|genre| genre.to_lowercase()).collect(),
					"type" => types = included,
					"status" => statuses = included,
					_ => {}
				}
			}
		}
		let query = query.map(|query| query.to_lowercase());
		// A pasted series link finds that series
		let pasted = query
			.as_deref()
			.and_then(|query| query.strip_prefix(params.base_url.as_ref()))
			.filter(|path| path.starts_with("/series/"))
			.map(|path| path.trim_end_matches('/'));

		let html = Request::get(format!("{}/series/", params.base_url))?.html()?;
		let entries = html
			.select("#searched_series_page > button")
			.map(|cards| {
				cards
					.filter(|card| {
						let matches_query = match pasted {
							Some(path) => card
								.select_first("a[href]")
								.and_then(|link| link.attr("href"))
								.is_some_and(|href| href.trim_end_matches('/') == path),
							// The card's title also carries the series' alternative titles
							None => query.as_ref().is_none_or(|query| {
								card.attr("title")
									.is_some_and(|title| title.to_lowercase().contains(query))
							}),
						};
						// A JSON list such as `["Drama","Sports"]` with apostrophes as `___`; a name
						// can hold a comma or stray spaces, so it splits between quotes and trims
						let has_genres = genres.is_empty()
							|| card.attr("tags").is_some_and(|tags| {
								let tags: Vec<String> = tags
									.trim_matches(['[', ']', '"'])
									.split("\",\"")
									.map(|tag| tag.trim().replace("___", "'").to_lowercase())
									.collect();
								genres.iter().all(|genre| tags.contains(genre))
							});
						let has = |values: &[String], attr: &str| {
							values.is_empty()
								|| card.attr(attr).is_some_and(|value| {
									values
										.iter()
										.any(|wanted| wanted.eq_ignore_ascii_case(&value))
								})
						};
						matches_query
							&& has_genres && has(&types, "data-type")
							&& has(&statuses, "data-status")
					})
					.filter_map(|card| {
						manga(&card.select_first("a[href]")?, &card, &params.base_url)
					})
					.collect()
			})
			.unwrap_or_default();
		Ok(MangaPageResult {
			entries,
			has_next_page: false,
		})
	}

	fn get_manga_update(
		&self,
		params: &Params,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{}{}", params.base_url, manga.key);
		let html = Request::get(&url)?.html()?;
		// An unknown series redirects to a not-found page without one
		let Some(title) = html.select_first("div.grid > h1").and_then(|h1| h1.text()) else {
			bail!("Series not found");
		};

		if needs_details {
			manga.title = title;
			manga.cover = html
				.select_first("[style*=photoURL]")
				.and_then(|cover| background_image(&cover))
				.or(manga.cover);
			manga.description = html
				.select_first("#expand_content")
				.and_then(|description| description.text())
				.map(|description| description.trim().escape_markdown())
				.filter(|description| !description.is_empty());
			manga.authors = labelled(&html, "Author").map(|author| names(&author));
			manga.artists = labelled(&html, "Artist").map(|artist| names(&artist));
			manga.status = labelled(&html, "Status")
				.map(|text| status(&text))
				.unwrap_or_default();
			let tags: Vec<String> = html
				.select("a[href*='genre=']")
				.map(|links| {
					links
						.filter_map(|link| link.text())
						.map(|tag| tag.trim_matches([',', ' ']).into())
						.filter(|tag: &String| !tag.is_empty())
						.collect()
				})
				.unwrap_or_default();
			// The page links only a series' first genres, while its keywords start with all of
			// them, so the rating reads both
			let keywords = html
				.select_first("meta[name=keywords]")
				.and_then(|meta| meta.attr("content"));
			manga.content_rating = content_rating(
				tags.iter().map(String::as_str).chain(
					keywords
						.iter()
						.flat_map(|words| words.split(',').map(str::trim)),
				),
			);
			manga.viewer = labelled(&html, "Type")
				.map(|kind| viewer(&kind))
				.unwrap_or_default();
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url);
		}

		if needs_chapters {
			manga.chapters = html.select("#chapters > a").map(|rows| {
				rows.filter(|row| {
					// Coin chapters need a login, and upcoming ones can't be read yet
					row.select_first("img[alt=Coin]").is_none()
						&& !row.select("span").is_some_and(|mut spans| {
							spans.any(|span| {
								span.own_text()
									.is_some_and(|text| text.trim() == "Upcoming")
							})
						})
				})
				.filter_map(|row| chapter(&row, &params.base_url))
				.collect()
			});
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let response = Request::get(format!("{}{}", params.base_url, chapter.key))?.string()?;
		let html = Html::parse(&response)?;
		if let Some(reader) = html.select_first("div.tiptap.novel-reader") {
			let Some(text) = novel_text(&reader) else {
				bail!("No text found");
			};
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}
		// The reader builds each page's url from this template and the image's uid
		let template = response
			.split_once("realUrl")
			.and_then(|(_, rest)| rest.split_once('`'))
			.and_then(|(_, rest)| rest.split_once('`'))
			.map(|(template, _)| template)
			.ok_or_else(|| error!("No image host found"))?;
		let pages = html
			.select("#pages > img")
			.map(|images| {
				images
					.map(|image| {
						let uid = image
							.attr("uid")
							.filter(|uid| !uid.is_empty())
							.ok_or_else(|| error!("A page has no image id"))?;
						Ok(Page {
							content: PageContent::url(template.replace("${uid}", &uid)),
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
		let mut components = Vec::new();

		let featured: Vec<Manga> = html
			.select("a.splide__slide[href*='/series/']")
			.map(|slides| {
				slides
					.filter_map(|slide| manga(&slide, &slide, &params.base_url))
					.collect()
			})
			.unwrap_or_default();
		if !featured.is_empty() {
			// Some sites feature 2:1 banners rather than covers. The class is compared here, since the
			// app's selector engine misreads the brackets inside a quoted attribute value
			let value = if html
				.select("a.splide__slide > div")
				.is_some_and(|mut slides| {
					slides.any(|slide| {
						slide
							.attr("class")
							.is_some_and(|class| class.contains("aspect-[2/1]"))
					})
				}) {
				HomeComponentValue::ImageScroller {
					links: featured
						.into_iter()
						.map(
							|Manga {
							     key, title, cover, ..
							 }| Link {
								title,
								image_url: cover,
								value: Some(LinkValue::Manga(Manga {
									key,
									..Default::default()
								})),
								..Default::default()
							},
						)
						.collect(),
					auto_scroll_interval: Some(5.0),
					width: Some(400),
					height: Some(200),
				}
			} else {
				HomeComponentValue::Scroller {
					entries: featured.into_iter().map(Link::from).collect(),
					listing: None,
				}
			};
			components.push(HomeComponent {
				title: None,
				subtitle: None,
				value,
			});
		}

		for heading in html.select("h2").into_iter().flatten() {
			let Some(title) = heading.text() else {
				continue;
			};
			// The site repeats some rows for its mobile layout, and the bulk discount row only
			// sells coin chapters
			if components
				.iter()
				.any(|component| component.title.as_ref() == Some(&title))
				|| title == "Bulk Discounted Series"
				|| params.hidden_rows.contains(&title.as_str())
			{
				continue;
			}
			let Some(section) = section(&heading) else {
				continue;
			};
			let value = if section.select_first(".editor-pick-button").is_some() {
				let entries: Vec<Manga> = section
					.select(".editor-pick-button")
					.map(|picks| picks.filter_map(|pick| editor_pick(&pick)).collect())
					.unwrap_or_default();
				if entries.is_empty() {
					continue;
				}
				HomeComponentValue::BigScroller {
					entries,
					auto_scroll_interval: None,
				}
			} else if title == "Latest Updates" {
				let entries: Vec<MangaWithChapter> = section
					.select(".latest-poster")
					.map(|posters| {
						posters
							.filter_map(|poster| newest_chapter(&poster, &params.base_url))
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
				// Pinned Series shares the latest row's posters
				let entries: Vec<Link> = section
					.select("button[title], .latest-poster")
					.map(|cards| {
						cards
							.filter_map(|card| {
								manga(
									&card.select_first("a[href*='/series/']")?,
									&card,
									&params.base_url,
								)
							})
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

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(params.base_url.as_ref()) else {
			return Ok(None);
		};
		// "/series/" alone, or with a query, is the catalogue rather than a series
		let is_series = |path: &str| {
			path.strip_prefix("/series/")
				.is_some_and(|slug| !slug.is_empty() && !slug.starts_with('?'))
		};
		if is_series(path) {
			return Ok(Some(DeepLinkResult::Manga { key: path.into() }));
		}
		if path.starts_with("/chapter/") {
			// A chapter path holds the series id, not the path its page lives at
			let html = Request::get(&url)?.html()?;
			let Some(series) = html.select("a[href*='/series/']").and_then(|mut links| {
				links.find_map(|link| {
					let href = link.attr("href")?;
					let path = href.strip_prefix_or_self(&params.base_url);
					is_series(path).then(|| path.into())
				})
			}) else {
				return Ok(None);
			};
			return Ok(Some(DeepLinkResult::Chapter {
				manga_key: series,
				key: path.into(),
			}));
		}
		Ok(None)
	}
}
