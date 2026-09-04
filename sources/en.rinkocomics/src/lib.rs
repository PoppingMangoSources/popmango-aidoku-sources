#![no_std]
use aidoku::{
	Chapter, ContentRating, Filter, FilterValue, HomeComponent, HomeComponentValue, HomeLayout,
	Manga, MangaPageResult, MangaWithChapter, MultiSelectFilter, Page, PageContent, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{element::ElementHelpers, string::StripPrefixOrSelf, uri::QueryParameters},
	imports::{
		html::{Element, Html},
		net::Request,
		std::send_partial_result,
	},
	prelude::*,
};
use madara::{Impl, Madara, Params, helpers::ElementImageAttr};

mod chapters;

use chapters::LOCK_SUFFIX;

const BASE_URL: &str = "https://rinkocomics.com";

/// Query values behind the sort filter, in the order `filters.json` lists them.
const SORT_IDS: [&str; 4] = ["newest", "oldest", "az", "za"];

struct RinkoComics;

/// Flattens a novel chapter's markup, keeping the paragraph breaks the reader
/// needs since the page carries one long block of prose.
fn prose_text(html: &str) -> String {
	let normalized = html
		.replace("<br>", "\n")
		.replace("<br/>", "\n")
		.replace("<br />", "\n")
		.replace("</p>", "\n\n");
	let mut out = String::with_capacity(normalized.len());
	let mut in_tag = false;
	for character in normalized.chars() {
		match character {
			'<' => in_tag = true,
			'>' => in_tag = false,
			_ if !in_tag => out.push(character),
			_ => {}
		}
	}
	out.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&#39;", "'")
		.replace("&nbsp;", " ")
		.trim()
		.to_string()
}

fn image_from(element: &Element, selector: &str) -> Option<String> {
	let image = element.select_first(selector)?;
	image
		.attr("abs:data-src")
		.or_else(|| image.attr("abs:data-lazy-src"))
		.or_else(|| image.attr("abs:src"))
		.or_else(|| image.attr("src"))
}

fn card_manga(
	params: &Params,
	element: &Element,
	link_selector: &str,
	title_selector: &str,
	image_selector: &str,
) -> Option<Manga> {
	let href = if link_selector.is_empty() {
		element.attr("abs:href").or_else(|| element.attr("href"))?
	} else {
		element
			.select_first(link_selector)?
			.attr("abs:href")
			.or_else(|| element.select_first(link_selector)?.attr("href"))?
	};
	let title = element
		.select_first(title_selector)
		.and_then(|el| el.text())
		.or_else(|| {
			element
				.select_first(link_selector)
				.and_then(|el| el.attr("title"))
		})?;
	// Every card variant on the home page marks its genres and latest chapter
	// the same way, so pick them up wherever they happen to be present.
	let tags: Vec<String> = element
		.select(
			".comic-genres .genre, .comic-genres-popular span, .novel-genres .genre-tag, .genre",
		)
		.map(|genres| {
			genres
				.filter_map(|genre| genre.text())
				.map(|genre| genre.trim().to_string())
				.filter(|genre| !genre.is_empty())
				.collect()
		})
		.unwrap_or_default();
	let description = element
		.select_first(".chapter-badge")
		.and_then(|badge| badge.text())
		.map(|text| text.trim().to_string())
		.filter(|text| !text.is_empty());

	Some(Manga {
		key: href.strip_prefix_or_self(&params.base_url).into(),
		title: title.trim().to_string(),
		cover: image_from(element, image_selector),
		description,
		content_rating: ContentRating::Safe,
		tags: (!tags.is_empty()).then_some(tags),
		url: Some(href),
		..Default::default()
	})
}

fn chapter_number(title: &str) -> Option<f32> {
	let mut number = String::new();
	for ch in title.chars() {
		if ch.is_ascii_digit() || (ch == '.' && !number.is_empty()) {
			number.push(ch);
		} else if !number.is_empty() {
			break;
		}
	}
	number.trim_matches('.').parse().ok()
}

impl Impl for RinkoComics {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			source_path: "comic".into(),
			..Default::default()
		}
	}

	/// The theme browses at `/comic/` with its own card markup and asks for the
	/// `comic` post type, so none of the Madara search defaults apply.
	fn get_search_manga_list(
		&self,
		params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut qs = QueryParameters::new();
		qs.push("post_type", Some("comic"));
		if let Some(query) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
			qs.push("s", Some(query));
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(value) = SORT_IDS.get(index as usize) {
						qs.set("sort", Some(value));
					}
				}
				FilterValue::MultiSelect { id, included, .. } if id == "genres[]" => {
					for genre in included {
						qs.push("genres[]", Some(&genre));
					}
				}
				FilterValue::Select { id, value } => qs.set(&id, Some(&value)),
				_ => {}
			}
		}

		let path = if page <= 1 {
			String::from("/comic/")
		} else {
			format!("/comic/page/{page}/")
		};
		let html = Request::get(format!("{}{path}?{qs}", params.base_url))?
			.header("Referer", &format!("{}/", params.base_url))
			.html()?;

		let entries = html
			.select("article.ac-card")
			.map(|cards| {
				cards
					.filter_map(|card| {
						let link = card.select_first(".ac-title a")?;
						let href = link.attr("abs:href")?;
						Some(Manga {
							key: href.strip_prefix_or_self(&params.base_url).into(),
							title: link.text()?.trim().into(),
							cover: image_from(&card, ".ac-thumb img"),
							url: Some(href),
							..Default::default()
						})
					})
					.collect()
			})
			.unwrap_or_default();

		Ok(MangaPageResult {
			entries,
			has_next_page: html.select_first(".ac-pagination a.next").is_some(),
		})
	}

	/// The theme puts its genre checkboxes on the browse page rather than the
	/// Madara search form, and labels them with a slug the browse query takes.
	fn get_dynamic_filters(&self, params: &Params) -> Result<Vec<Filter>> {
		let (options, ids): (Vec<_>, Vec<_>) = Request::get(format!("{}/comic/", params.base_url))?
			.header("Referer", &format!("{}/", params.base_url))
			.html()?
			.select(".ac-filter-group.ac-genre input[name='genres[]']")
			.map(|inputs| {
				inputs
					.filter_map(|input| {
						let id = input.attr("value")?;
						let option = input.parent()?.select_first(".ac-option-text")?.text()?;
						Some((option.into(), id.into()))
					})
					.unzip()
			})
			.unwrap_or_default();

		Ok(vec![
			MultiSelectFilter {
				id: "genres[]".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: false,
				options,
				ids: Some(ids),
				..Default::default()
			}
			.into(),
		])
	}

	/// The site paginates chapters through its own ajax action instead of the
	/// Madara endpoint, so the chapter list is collected here.
	fn get_manga_update(
		&self,
		params: &Params,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let url = format!("{}{}", params.base_url, manga.key);
		let body = Request::get(&url)?
			.header("Referer", &format!("{}/", params.base_url))
			.string()?;
		let html = Html::parse_with_url(&body, &url)?;

		if needs_details {
			manga.title = html
				.select_first(&params.details_title_selector)
				.and_then(|el| el.own_text())
				.unwrap_or(manga.title);
			manga.cover = html
				.select_first(&params.details_cover_selector)
				.and_then(|img| img.img_attr(params.use_style_images))
				.or(manga.cover);
			manga.artists = html.select(&params.details_artist_selector).map(|els| {
				els.filter_map(|span| span.text())
					.filter(|name| !name.is_empty())
					.collect()
			});
			manga.authors = html.select(&params.details_author_selector).map(|els| {
				els.filter_map(|span| span.text())
					.filter(|name| !name.is_empty())
					.collect()
			});
			manga.description = html
				.select_first(&params.details_description_selector)
				.and_then(|div| div.text_with_newlines())
				.map(|text| text.trim().into());
			manga.tags = html
				.select(&params.details_tag_selector)
				.map(|els| els.filter_map(|el| el.text()).collect());
			manga.url = Some(url.clone());
			manga.status = html
				.select_first(&params.details_status_selector)
				.and_then(|span| span.text())
				.map(|text| self.get_manga_status(&text))
				.unwrap_or_default();
			manga.content_rating = self.get_manga_content_rating(&html, &manga);
			manga.viewer = html
				.select_first(&params.details_type_selector)
				.and_then(|el| el.own_text())
				.map(|text| self.get_manga_viewer(&text, params.default_viewer))
				.unwrap_or(params.default_viewer);

			// Walking the ajax pages below costs a request per batch, so hand the
			// details over before starting it.
			if needs_chapters {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			manga.chapters = Some(chapters::parse_chapters(&body, &html, &params.base_url)?);
		}

		Ok(manga)
	}

	/// Novels sit alongside comics here and serve prose where a comic serves
	/// page images, so neither the madara page list nor its chapter protector
	/// applies.
	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if chapter.key.ends_with(LOCK_SUFFIX) {
			bail!("This chapter is locked. Unlock it on the website to read it.");
		}

		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?
			.header("Referer", &format!("{}/", params.base_url))
			.html()?;

		let images: Vec<Page> = html
			.select("img.chapter-image")
			.map(|elements| {
				elements
					.filter_map(|element| {
						element
							.attr("abs:data-src")
							.or_else(|| element.attr("abs:src"))
					})
					.map(|url| Page {
						content: PageContent::url(url),
						..Default::default()
					})
					.collect()
			})
			.unwrap_or_default();
		if !images.is_empty() {
			return Ok(images);
		}

		let prose = html
			.select_first("#textContent")
			.or_else(|| html.select_first(".novel-content"))
			.and_then(|element| element.html())
			.map(|html| prose_text(&html))
			.filter(|text| !text.is_empty());
		let Some(prose) = prose else {
			bail!("No pages or text found for this chapter");
		};
		Ok(vec![Page {
			content: PageContent::text(prose),
			..Default::default()
		}])
	}
	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		let html = Request::get(&params.base_url)?.html()?;
		let mut components = Vec::new();

		let featured: Vec<aidoku::Link> = html
			.select(".hero-slider .slide")
			.map(|items| {
				items
					.filter_map(|item| {
						let manga = card_manga(params, &item, "a", ".comic-title", "img")?;
						let genres = manga
							.tags
							.as_ref()
							.filter(|genres| !genres.is_empty())
							.map(|genres| genres.join(" · "));
						let mut link = aidoku::Link::from(manga);
						link.subtitle = genres;
						Some(link)
					})
					.collect()
			})
			.unwrap_or_default();
		if !featured.is_empty() {
			components.push(HomeComponent {
				title: Some("Featured".into()),
				subtitle: None,
				value: HomeComponentValue::ImageScroller {
					links: featured,
					auto_scroll_interval: Some(6.0),
					width: None,
					height: None,
				},
			});
		}

		let hot: Vec<aidoku::Link> = html
			.select(".popular-comics .comic-card-popular")
			.map(|items| {
				items
					.filter_map(|item| {
						card_manga(
							params,
							&item,
							"a.read-btn",
							".comic-title-popular",
							".comic-cover img",
						)
					})
					.map(Into::into)
					.collect()
			})
			.unwrap_or_default();
		if !hot.is_empty() {
			components.push(HomeComponent {
				title: Some("Hot This Week".into()),
				subtitle: None,
				value: HomeComponentValue::MangaList {
					ranking: true,
					page_size: Some(5),
					entries: hot,
					listing: None,
				},
			});
		}

		let pinned: Vec<Manga> = html
			.select("a.pinned-comic-card")
			.map(|items| {
				items
					.filter_map(|item| {
						card_manga(
							params,
							&item,
							"",
							".pinned-comic-title",
							".comic-thumbnail img",
						)
					})
					.take(12)
					.collect()
			})
			.unwrap_or_default();
		if !pinned.is_empty() {
			components.push(HomeComponent {
				title: Some("Editor's Choice".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: pinned,
					auto_scroll_interval: None,
				},
			});
		}

		let latest: Vec<MangaWithChapter> = html
			.select(".latest-releases .comic-card")
			.map(|items| {
				items
					.filter_map(|item| {
						let manga = card_manga(
							params,
							&item,
							"a.comic-card__cover",
							".comic-card__title",
							".comic-card__cover img",
						)?;
						let chapter = item.select_first(
							"a.chapter-item[href*='/chapter/']:not(.locked-chapter):not(.is-locked)",
						)?;
						let href = chapter.attr("abs:href").or_else(|| chapter.attr("href"))?;
						let label = chapter
							.select_first("label")
							.and_then(|el| el.text())
							.or_else(|| chapter.text())?;
						let date_uploaded = chapter
							.select_first(".chapter-date, time")
							.or_else(|| item.select_first(".chapter-date, time"))
							.and_then(|el| el.attr("datetime").or_else(|| el.text()))
							.and_then(|text| chapters::chapter_date(text.trim()));
						Some(MangaWithChapter {
							manga,
							chapter: Chapter {
								key: href.strip_prefix_or_self(&params.base_url).into(),
								chapter_number: chapter_number(&label),
								date_uploaded,
								title: Some(label),
								url: Some(href),
								..Default::default()
							},
						})
					})
					.collect()
			})
			.unwrap_or_default();
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Releases".into()),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: None,
				},
			});
		}

		let novels: Vec<aidoku::Link> = html
			.select(".novels-section .novel-card")
			.map(|items| {
				items
					.filter_map(|item| {
						let mut manga = card_manga(
							params,
							&item,
							"a.novel-card-link",
							".novel-title",
							".novel-cover img",
						)?;
						let chapters = item
							.select_first(".chapter-badge")
							.and_then(|el| el.text())
							.map(|text| text.trim().to_string())
							.filter(|text| !text.is_empty());
						let blurb = item
							.select_first(".novel-excerpt")
							.and_then(|el| el.text())
							.map(|text| text.trim().to_string())
							.filter(|text| !text.is_empty());
						manga.description = match (chapters, blurb) {
							(Some(chapters), Some(blurb)) => Some(format!("{chapters} · {blurb}")),
							(chapters, blurb) => chapters.or(blurb),
						};
						Some(manga)
					})
					.map(Into::into)
					.collect()
			})
			.unwrap_or_default();
		if !novels.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Novels".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: novels,
					listing: None,
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

register_source!(
	Madara<RinkoComics>,
	Home,
	DynamicFilters,
	DeepLinkHandler,
	ImageRequestProvider
);
