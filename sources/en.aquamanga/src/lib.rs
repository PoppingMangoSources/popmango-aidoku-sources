#![no_std]

use aidoku::{
	Chapter, DeepLinkResult, FilterValue, HomeComponent, HomeComponentValue, HomeLayout, Link,
	Listing, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Result, Source, Viewer,
	alloc::{String, Vec},
	helpers::string::{PlainText, StripPrefixOrSelf},
	imports::{html::Element, net::Request, std::send_partial_result},
	prelude::*,
};
use madara::{
	Impl, LoadMoreStrategy, Madara, Params,
	helpers::{self as madara_helpers, ElementImageAttr},
};

mod helpers;

use helpers::*;

struct AquaManga;

impl Impl for AquaManga {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			use_new_chapter_endpoint: true,
			use_load_more_request: LoadMoreStrategy::Never,
			default_viewer: Viewer::Webtoon,
			details_title_selector: ".aqua-series-info__title".into(),
			details_cover_selector: ".aqua-series-cover__img".into(),
			details_author_selector: ".aqua-sp-fact a[href*='/manga-author/']".into(),
			details_artist_selector: ".aqua-sp-fact a[href*='/manga-artist/']".into(),
			details_description_selector: ".aqua-series-synopsis".into(),
			details_tag_selector: ".aqua-series-genre-pill".into(),
			details_status_selector: ".aqua-sp-status".into(),
			details_type_selector: ".aqua-sp-tag".into(),
			chapter_selector: "a.aqua-ch-item".into(),
			..Default::default()
		}
	}

	fn get_search_manga_list(
		&self,
		_params: &Params,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut sort = SORTS[0];
		let mut status = None;
		let mut genre = None;
		let mut ch_min = None;
		let mut ch_max = None;

		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(&s) = SORTS.get(index as usize) {
						sort = s;
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => match id.as_str() {
					"manga_status" => status = Some(value),
					"genre" => genre = Some(value),
					_ => {}
				},
				FilterValue::Range { id, from, to } if id == "chapters" => {
					ch_min = from;
					ch_max = to;
				}
				_ => {}
			}
		}

		fetch_archive(ArchiveQuery {
			page,
			query: query.as_deref(),
			sort,
			status: status.as_deref(),
			genre: genre.as_deref(),
			ch_min,
			ch_max,
		})
	}

	fn get_manga_list(
		&self,
		_params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let sort = match listing.id.as_str() {
			"latest" | "popular" | "views" | "new-manga" => listing.id.as_str(),
			_ => bail!("Unknown listing"),
		};
		fetch_archive(ArchiveQuery {
			page,
			query: None,
			sort,
			status: None,
			genre: None,
			ch_min: None,
			ch_max: None,
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
		let html = self.modify_request(params, Request::get(&url)?)?.html()?;

		if needs_details {
			if let Some(title) = html
				.select_first(&params.details_title_selector)
				.and_then(|el| el.text())
				&& !title.is_empty()
			{
				manga.title = title;
			}
			manga.cover = html
				.select_first(&params.details_cover_selector)
				.and_then(|img| img.img_attr(false))
				.or(manga.cover);
			manga.authors = credit_list(&html, &params.details_author_selector);
			manga.artists = credit_list(&html, &params.details_artist_selector);
			manga.description = html
				.select_first(&params.details_description_selector)
				.and_then(|div| {
					let paragraphs: Vec<String> = div
						.select("p")
						.map(|els| {
							els.filter_map(|p| p.text())
								.filter(|s| !s.is_empty())
								.map(|s| s.escape_markdown())
								.collect()
						})
						.unwrap_or_default();
					if !paragraphs.is_empty() {
						Some(paragraphs.join("\n\n"))
					} else {
						div.text()
							.and_then(|s| clean_excerpt(&s, &manga.title))
							.map(|s| s.escape_markdown())
					}
				});
			let mut tags: Vec<String> = html
				.select(&params.details_tag_selector)
				.map(|els| {
					els.filter_map(|el| el.text())
						.filter(|s| !s.is_empty())
						.collect()
				})
				.unwrap_or_default();
			tags.sort_unstable();
			tags.dedup();
			manga.content_rating = content_rating(&tags);
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url.clone());
			manga.status = html
				.select_first(&params.details_status_selector)
				.and_then(|el| el.text())
				.map(|s| self.get_manga_status(&s))
				.unwrap_or_default();
			let type_label = html
				.select_first(&params.details_type_selector)
				.and_then(|el| el.text());
			manga.viewer = viewer(type_label.as_deref());
		}

		if needs_chapters {
			let mut chapter_elements = html
				.select(&params.chapter_selector)
				.ok_or_else(|| error!("Invalid chapter selector"))?;
			if chapter_elements.is_empty() {
				if needs_details {
					send_partial_result(&manga);
				}
				let ajax_url = format!("{}/ajax/chapters", url.trim_end_matches('/'));
				let ajax_html = Request::post(ajax_url)?
					.header("Referer", &format!("{}/", params.base_url))
					.header("X-Requested-With", "XMLHttpRequest")
					.html()?;
				chapter_elements = ajax_html
					.select(&params.chapter_selector)
					.ok_or_else(|| error!("Invalid chapter selector"))?;
			}
			manga.chapters = Some(
				chapter_elements
					.filter_map(|el| self.parse_chapter_element(params, el))
					.collect(),
			);
		}

		Ok(manga)
	}

	fn get_manga_status(&self, str: &str) -> MangaStatus {
		match str.to_ascii_lowercase().as_str() {
			"ongoing" | "serialization" => MangaStatus::Ongoing,
			"completed" => MangaStatus::Completed,
			"on hold" | "hiatus" => MangaStatus::Hiatus,
			"cancelled" | "canceled" | "dropped" => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		}
	}

	fn parse_chapter_element(&self, params: &Params, element: Element) -> Option<Chapter> {
		let url = element.attr("abs:href")?;
		let raw_title = element.select_first(".aqua-ch-item__name")?.text()?;
		let chapter_number = madara_helpers::find_first_f32(&raw_title);
		let date_uploaded = element
			.select_first(".aqua-ch-item__time")
			.and_then(|el| el.text())
			.and_then(|d| parse_chapter_date(&d, params));

		Some(Chapter {
			key: url.strip_prefix_or_self(&params.base_url).into(),
			title: clean_chapter_title(&raw_title, chapter_number),
			chapter_number,
			date_uploaded,
			url: Some(url),
			..Default::default()
		})
	}

	fn get_home(&self, params: &Params) -> Result<HomeLayout> {
		let html = self
			.modify_request(params, Request::get(&params.base_url)?)?
			.html()?;
		let mut components = Vec::new();

		let hero: Vec<Manga> = html
			.select("#aqua-hero-slider article.aqua-hero-slide")
			.map(|els| {
				els.filter_map(|slide| {
					let link = slide.select_first(".aqua-hero-slide__title a")?;
					let key = link
						.attr("href")?
						.strip_prefix_or_self(&params.base_url)
						.into();
					let title = link.text()?;
					if title.is_empty() {
						return None;
					}
					let cover = slide
						.select_first("img.aqua-hero-slide__cover")
						.and_then(|img| img.img_attr(false));
					let description = slide
						.select_first(".aqua-hero-slide__excerpt")
						.and_then(|el| el.text())
						.and_then(|s| clean_excerpt(&s, &title));
					let mut tags: Vec<String> = slide
						.select(".aqua-hero-slide__genre")
						.map(|els| {
							els.filter_map(|el| el.text())
								.filter(|s| !s.is_empty())
								.collect()
						})
						.unwrap_or_default();
					tags.sort_unstable();
					tags.dedup();
					let rating = content_rating(&tags);
					let type_badge = slide
						.select_first(".aqua-hero-slide__type-badge")
						.and_then(|el| el.text());
					Some(Manga {
						key,
						title,
						cover,
						description,
						content_rating: rating,
						viewer: viewer(type_badge.as_deref()),
						tags: (!tags.is_empty()).then_some(tags),
						..Default::default()
					})
				})
				.collect()
			})
			.unwrap_or_default();
		if !hero.is_empty() {
			components.push(HomeComponent {
				title: None,
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: hero,
					auto_scroll_interval: Some(8.0),
				},
			});
		}

		let parse_scroll_card = |el: Element, title_sel: &str, cover_sel: &str| -> Option<Link> {
			let key = el
				.attr("href")?
				.strip_prefix_or_self(&params.base_url)
				.into();
			let title = el.select_first(title_sel)?.text()?;
			if title.is_empty() {
				return None;
			}
			let cover = el
				.select_first(cover_sel)
				.and_then(|img| img.img_attr(false));
			Some(
				Manga {
					key,
					title,
					cover,
					..Default::default()
				}
				.into(),
			)
		};

		let pinned: Vec<Link> = html
			.select("#aqua-pinned-scroll a.aqua-pinned-card")
			.map(|els| {
				els.filter_map(|el| {
					parse_scroll_card(
						el,
						".aqua-popular-card__title",
						"img.aqua-popular-card__cover",
					)
				})
				.collect()
			})
			.unwrap_or_default();
		if !pinned.is_empty() {
			components.push(HomeComponent {
				title: Some(String::from("Pinned")),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: pinned,
					listing: None,
				},
			});
		}

		let popular_today: Vec<Link> = html
			.select("#aqua-popular-scroll a.aqua-popular-card")
			.map(|els| {
				els.filter_map(|el| {
					parse_scroll_card(
						el,
						".aqua-popular-card__title",
						"img.aqua-popular-card__cover",
					)
				})
				.collect()
			})
			.unwrap_or_default();
		if !popular_today.is_empty() {
			components.push(HomeComponent {
				title: Some(String::from("Popular today")),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: popular_today,
					listing: Some(listing("popular", "Popular")),
				},
			});
		}

		let latest: Vec<MangaWithChapter> = html
			.select(".aqua-manga-grid.aqua-latest-view article.aqua-manga-card")
			.map(|els| {
				els.filter_map(|item| {
					let manga = parse_archive_card(&item)?;
					let chapter_link = item.select_first(".aqua-manga-card__chapters a")?;
					let chapter_key = chapter_link
						.attr("href")?
						.strip_prefix_or_self(&params.base_url)
						.into();
					let raw_name = chapter_link
						.select_first(".aqua-chapter-pill__name")
						.and_then(|el| el.text())?;
					let chapter_number = madara_helpers::find_first_f32(&raw_name);
					let date_uploaded = chapter_link
						.select_first(".aqua-chapter-pill__time")
						.and_then(|el| el.text())
						.and_then(|d| parse_chapter_date(&d, params));
					Some(MangaWithChapter {
						manga,
						chapter: Chapter {
							key: chapter_key,
							title: clean_chapter_title(&raw_name, chapter_number),
							chapter_number,
							date_uploaded,
							..Default::default()
						},
					})
				})
				.collect()
			})
			.unwrap_or_default();
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some(String::from("Latest updates")),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: Some(listing("latest", "Latest")),
				},
			});
		}

		let new_series: Vec<Link> = html
			.select("#aqua-new-series-scroll a.aqua-manga-card--new")
			.map(|els| {
				els.filter_map(|el| {
					parse_scroll_card(el, ".aqua-manga-card__title", "img.aqua-manga-card__cover")
				})
				.collect()
			})
			.unwrap_or_default();
		if !new_series.is_empty() {
			components.push(HomeComponent {
				title: Some(String::from("New Series")),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: new_series,
					listing: Some(listing("new-manga", "Newest")),
				},
			});
		}

		let popular: Vec<Link> = html
			.select(
				"ol.aqua-ranking__list[data-range='week'] li.aqua-rank-item a.aqua-rank-item__link",
			)
			.map(|els| {
				els.filter_map(|el| parse_scroll_card(el, ".aqua-rank-item__title", "img"))
					.collect()
			})
			.unwrap_or_default();
		if !popular.is_empty() {
			components.push(HomeComponent {
				title: Some(String::from("Popular")),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: popular,
					listing: Some(listing("views", "Most Views")),
				},
			});
		}

		components.push(HomeComponent {
			title: Some(String::from("Browse")),
			subtitle: None,
			value: HomeComponentValue::Filters(browse_filters()),
		});

		Ok(HomeLayout { components })
	}

	fn handle_deep_link(&self, params: &Params, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(raw_path) = url.strip_prefix(params.base_url.as_ref()) else {
			return Ok(None);
		};
		let path = raw_path.split(['?', '#']).next().unwrap_or(raw_path);
		let mut parts = path.split('/').filter(|s| !s.is_empty());
		if parts.next() != Some("manga") {
			return Ok(None);
		}
		let Some(slug) = parts.next() else {
			return Ok(None);
		};
		let manga_key = format!("/manga/{slug}/");
		if parts.next().is_some() {
			let key = if path.ends_with('/') {
				String::from(path)
			} else {
				format!("{path}/")
			};
			Ok(Some(DeepLinkResult::Chapter { manga_key, key }))
		} else {
			Ok(Some(DeepLinkResult::Manga { key: manga_key }))
		}
	}
}

register_source!(
	Madara<AquaManga>,
	ListingProvider,
	Home,
	DeepLinkHandler,
	MigrationHandler,
	ImageRequestProvider
);
