use crate::{
	Params,
	helpers::{
		background_image, chapter_date, chapter_parts, has_badge, is_novel, labelled, manga, status,
	},
};
use aidoku::{
	Chapter, ContentRating, DeepLinkResult, FilterValue, HomeComponent, HomeComponentValue,
	HomeLayout, Link, Manga, MangaPageResult, MangaWithChapter, Page, PageContent, Result, Viewer,
	alloc::{String, Vec, vec},
	helpers::string::{PlainText, StripPrefixOrSelf},
	imports::{
		html::{Element, Html},
		net::Request,
	},
	prelude::*,
};
use base64::{Engine, engine::general_purpose::STANDARD};

pub trait Impl {
	fn new() -> Self;

	fn params(&self) -> Params;

	// The site sends its whole catalogue on one page and filters it in the browser, ignoring query
	// parameters, so the source filters it the same way.
	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		_page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut genres = Vec::new();
		let mut excluded_genres = Vec::new();
		let mut types = Vec::new();
		let mut statuses = Vec::new();
		for filter in filters {
			if let FilterValue::MultiSelect {
				id,
				included,
				excluded,
			} = filter
			{
				match id.as_str() {
					"genre" => {
						genres = included;
						excluded_genres = excluded;
					}
					"type" => types = included,
					"status" => statuses = included,
					_ => {}
				}
			}
		}
		let query = query.map(|query| query.to_lowercase());

		let html = Request::get(format!("{}/series/", params.base_url))?.html()?;
		let entries = html
			.select("#searched_series_page > button")
			.map(|cards| {
				cards
					.filter(|card| {
						// The card's title also carries the series' alternative titles.
						let title = card.attr("title").unwrap_or_default().to_lowercase();
						let tags: Vec<String> = card
							.attr("tags")
							.unwrap_or_default()
							.trim_matches(['[', ']'])
							.split(',')
							.map(|tag| {
								tag.trim()
									.trim_matches('"')
									.replace("___", "'")
									.to_lowercase()
							})
							.collect();
						let has = |values: &[String], attr: &str| {
							values.is_empty()
								|| card.attr(attr).is_some_and(|value| {
									values
										.iter()
										.any(|wanted| wanted.eq_ignore_ascii_case(&value))
								})
						};
						!is_novel(card)
							&& query.as_ref().is_none_or(|query| title.contains(query))
							&& genres
								.iter()
								.all(|genre| tags.contains(&genre.to_lowercase()))
							&& !excluded_genres
								.iter()
								.any(|genre| tags.contains(&genre.to_lowercase()))
							&& has(&types, "data-type")
							&& has(&statuses, "data-status")
					})
					.filter_map(|card| {
						manga(
							&card.select_first("a[href]")?,
							card.select_first("[style*=background-image]"),
							&params.base_url,
						)
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

		if needs_details {
			if let Some(title) = html.select_first("div.grid > h1").and_then(|h1| h1.text()) {
				manga.title = title;
			}
			manga.cover = html
				.select_first("[style*=photoURL]")
				.and_then(|cover| background_image(&cover))
				.or(manga.cover);
			manga.description = html
				.select_first("#expand_content")
				.and_then(|description| description.text())
				.map(|description| description.trim().escape_markdown())
				.filter(|description| !description.is_empty());
			manga.authors = labelled(&html, "Author").map(|author| vec![author]);
			manga.artists = labelled(&html, "Artist").map(|artist| vec![artist]);
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
			manga.content_rating = if tags
				.iter()
				.any(|tag| matches!(tag.as_str(), "Adult" | "Smut" | "Hentai"))
			{
				ContentRating::NSFW
			} else if tags
				.iter()
				.any(|tag| matches!(tag.as_str(), "Ecchi" | "Mature"))
			{
				ContentRating::Suggestive
			} else {
				ContentRating::Safe
			};
			manga.viewer = match labelled(&html, "Type").as_deref() {
				Some("manga") => Viewer::RightToLeft,
				Some("comic") => Viewer::LeftToRight,
				Some("manhwa" | "manhua" | "mangatoon") => Viewer::Webtoon,
				_ => Viewer::Unknown,
			};
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url);
		}

		if needs_chapters {
			manga.chapters = html.select("#chapters > a").map(|rows| {
				rows.filter(|row| {
					// Coin chapters need a login, and upcoming ones can't be read yet.
					row.select_first("img[alt=Coin]").is_none() && !has_badge(row, "Upcoming")
				})
				.filter_map(|row| chapter(&row, &params.base_url))
				.collect()
			});
		}

		Ok(manga)
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let response = Request::get(&url)?.string()?;
		// The reader builds each page's url from this template and the image's uid.
		let template = response
			.split_once("realUrl")
			.and_then(|(_, rest)| rest.split_once('`'))
			.and_then(|(_, rest)| rest.split_once('`'))
			.map(|(template, _)| template)
			.ok_or_else(|| error!("No image host found"))?;
		let pages: Vec<Page> = Html::parse(&response)?
			.select("#pages > img")
			.map(|images| {
				images
					.filter_map(|image| image.attr("uid"))
					.filter(|uid| !uid.is_empty())
					.map(|uid| Page {
						content: PageContent::url(template.replace("${uid}", &uid)),
						..Default::default()
					})
					.collect()
			})
			.unwrap_or_default();
		if pages.is_empty() {
			bail!("No pages found");
		}
		Ok(pages)
	}

	// Each row is an `h2` over its cards; the featured slider above them has no heading.
	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		let html = Request::get(params.base_url.as_ref())?.html()?;
		let mut components = Vec::new();

		let featured: Vec<Link> = html
			.select("a.splide__slide[href*='/series/']")
			.map(|slides| {
				slides
					.filter(|slide| !is_novel(slide))
					.filter_map(|slide| {
						manga(
							&slide,
							slide.select_first("[style*=background-image]"),
							&params.base_url,
						)
					})
					.map(Link::from)
					.collect()
			})
			.unwrap_or_default();
		if !featured.is_empty() {
			components.push(HomeComponent {
				title: None,
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: featured,
					listing: None,
				},
			});
		}

		let mut titles: Vec<String> = Vec::new();
		for heading in html.select("h2").into_iter().flatten() {
			let Some(title) = heading.text() else {
				continue;
			};
			// The site repeats some rows for its mobile layout; the bulk discount row only sells
			// coin chapters.
			if titles.contains(&title) || title == "Bulk Discounted Series" {
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
							.filter(|poster| !is_novel(poster))
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
				// Pinned Series shares the latest row's posters.
				let entries: Vec<Link> = section
					.select("button[title], .latest-poster")
					.map(|cards| {
						cards
							.filter(|card| !is_novel(card))
							.filter_map(|card| {
								manga(
									&card.select_first("a[href*='/series/']")?,
									card.select_first("[style*=background-image]"),
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
			titles.push(title.clone());
			components.push(HomeComponent {
				title: Some(title),
				subtitle: None,
				value,
			});
		}

		Ok(HomeLayout { components })
	}

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		// "/series/" alone, or with a query, is the catalogue rather than a series.
		let is_series = |path: &str| {
			path.strip_prefix("/series/")
				.is_some_and(|slug| !slug.is_empty() && !slug.starts_with('?'))
		};
		let path = url.strip_prefix_or_self(&params.base_url);
		if is_series(path) {
			return Ok(Some(DeepLinkResult::Manga { key: path.into() }));
		}
		if path.starts_with("/chapter/") {
			// A chapter path holds the series id, not the path its page lives at.
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

// A heading sits a level or two inside the grid that holds its cards.
fn section(heading: &Element) -> Option<Element> {
	let mut node = heading.parent();
	for _ in 0..3 {
		let element = node?;
		if element
			.select_first(".latest-poster, .editor-pick-button, button[title] a[href*='/series/']")
			.is_some()
		{
			return Some(element);
		}
		node = element.parent();
	}
	None
}

// A pick has no link; the site's script opens `/series/<data-id>`, shows the cover from the
// upload host and decodes the description from base64.
fn editor_pick(pick: &Element) -> Option<Manga> {
	let description = pick
		.attr("data-description")
		.and_then(|encoded| STANDARD.decode(encoded).ok())
		.and_then(|bytes| String::from_utf8(bytes).ok())
		.map(|text| text.trim().into())
		.filter(|text: &String| !text.is_empty());
	Some(Manga {
		key: format!("/series/{}/", pick.attr("data-id")?),
		title: pick.attr("title")?.trim().into(),
		cover: pick
			.attr("data-img")
			.map(|image| format!("https://wsrv.nl/?w=600&url=cdn.meowing.org/uploads/{image}")),
		description,
		..Default::default()
	})
}

fn chapter(link: &Element, base_url: &str) -> Option<Chapter> {
	let key: String = link.attr("href")?.strip_prefix_or_self(base_url).into();
	let (chapter_number, title) = chapter_parts(&link.attr("title")?);
	Some(Chapter {
		url: Some(format!("{base_url}{key}")),
		key,
		title,
		chapter_number,
		date_uploaded: link.attr("d").and_then(|date| chapter_date(&date)),
		..Default::default()
	})
}

// A latest row lists each series' newest chapters; coin chapters need a login.
fn newest_chapter(poster: &Element, base_url: &str) -> Option<MangaWithChapter> {
	let chapter = poster
		.select("a[href*='/chapter/']")?
		.filter(|link| link.select_first("img[alt=Coin]").is_none())
		.find_map(|link| chapter(&link, base_url))?;
	Some(MangaWithChapter {
		manga: manga(
			&poster.select_first("a[href*='/series/']")?,
			poster.select_first("[style*=background-image]"),
			base_url,
		)?,
		chapter,
	})
}
