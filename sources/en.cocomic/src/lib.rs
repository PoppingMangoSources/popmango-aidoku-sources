#![no_std]
mod helpers;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, Filter, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	Manga, MangaPageResult, MangaStatus, MultiSelectFilter, Page, PageContent, PageContext, Result,
	Source, Viewer,
	alloc::{String, Vec, vec},
	helpers::{element::ElementHelpers, string::StripPrefixOrSelf},
	imports::{
		html::{Document, Element},
		net::Request,
		std::send_partial_result,
	},
	prelude::*,
};
use helpers::{
	BASE_URL, card_manga, clean, clean_chapter_title, hero_entries, is_valid_genre, listing,
	swimlane_latest, swimlane_manga,
};
use madara::{
	Impl, LoadMoreStrategy, Madara, Params,
	helpers::{self as madara_helpers, ElementImageAttr},
};

struct Cocomic;

impl Impl for Cocomic {
	fn new() -> Self {
		Self
	}

	fn params(&self) -> Params {
		Params {
			base_url: BASE_URL.into(),
			use_new_chapter_endpoint: true,
			use_load_more_request: LoadMoreStrategy::Never,
			default_viewer: Viewer::Webtoon,
			genre_endpoint: "/?s=&post_type=wp-manga".into(),
			details_title_selector: "h1.asura-series-title".into(),
			details_cover_selector: "img.asura-poster-img".into(),
			details_author_selector: ".asura-chip-author .author-name-text".into(),
			details_artist_selector: ".artist-content > a".into(),
			details_description_selector: ".asura-synopsis-content".into(),
			details_tag_selector: "a.asura-genre-tag".into(),
			details_status_selector: ".asura-badge-status".into(),
			details_type_selector: ".asura-hero-badge-type".into(),
			chapter_selector: "li.wp-manga-chapter:not(.premium)".into(),
			..Default::default()
		}
	}

	fn apply_manga_details(&self, params: &Params, manga: &mut Manga, html: &Document, url: &str) {
		manga.title = html
			.select_first(&params.details_title_selector)
			.and_then(|e| e.own_text())
			.unwrap_or(core::mem::take(&mut manga.title));
		manga.cover = html
			.select_first(&params.details_cover_selector)
			.and_then(|img| img.img_attr(params.use_style_images))
			.or(manga.cover.take());
		manga.artists = html.select(&params.details_artist_selector).map(|els| {
			els.filter_map(|span| span.text())
				.filter(|t| !t.is_empty())
				.collect()
		});
		manga.authors = html.select(&params.details_author_selector).map(|els| {
			els.filter_map(|span| span.text())
				.filter(|t| !t.is_empty())
				.collect()
		});
		manga.description = html
			.select_first(&params.details_description_selector)
			.and_then(|div| div.text_with_newlines())
			.map(|t| t.trim().into())
			.filter(|t: &String| !t.is_empty());
		// Cocomic mixes chapter tags (`Ch.133`) into `a.asura-genre-tag`.
		let mut tags: Vec<String> = html
			.select(&params.details_tag_selector)
			.into_iter()
			.flatten()
			.filter_map(|el| el.text())
			.map(|t| clean(&t))
			.filter(|t| is_valid_genre(t))
			.collect();
		tags.sort_unstable();
		tags.dedup();
		manga.tags = (!tags.is_empty()).then_some(tags);
		manga.url = Some(url.into());
		manga.status = html
			.select_first(&params.details_status_selector)
			.and_then(|span| span.text())
			.map(|text| self.get_manga_status(&text))
			.unwrap_or_default();
		manga.content_rating = self.get_manga_content_rating(html, manga);
		manga.viewer = html
			.select_first(&params.details_type_selector)
			.and_then(|el| el.own_text())
			.map(|text| self.get_manga_viewer(&text, params.default_viewer))
			.unwrap_or(params.default_viewer);
		send_partial_result(manga);
	}

	fn parse_chapter_element(&self, params: &Params, element: Element) -> Option<Chapter> {
		let url_element = element.select_first(&params.chapter_url_selector)?;
		let url = url_element.attr("abs:href")?;
		let title_element = if params.chapter_title_selector == params.chapter_url_selector {
			url_element
		} else {
			element.select_first(&params.chapter_title_selector)?
		};
		let title_text = clean(&title_element.text()?);
		let chapter_number = madara_helpers::find_first_f32(&title_text);
		Some(Chapter {
			key: url.strip_prefix_or_self(&params.base_url).into(),
			title: clean_chapter_title(&title_text, chapter_number),
			chapter_number,
			date_uploaded: element
				.select_first(&params.chapter_date_selector)
				.and_then(|time| time.text())
				.map(|date| madara_helpers::parse_chapter_date(params, &date)),
			url: Some(url),
			..Default::default()
		})
	}

	fn get_manga_status(&self, str: &str) -> MangaStatus {
		match str.trim().to_ascii_lowercase().as_str() {
			"ongoing" | "updating" => MangaStatus::Ongoing,
			"completed" => MangaStatus::Completed,
			"on hold" | "hiatus" => MangaStatus::Hiatus,
			"canceled" | "cancelled" | "dropped" => MangaStatus::Cancelled,
			_ => MangaStatus::Unknown,
		}
	}

	fn get_manga_content_rating(&self, _html: &Document, manga: &Manga) -> ContentRating {
		let Some(ref tags) = manga.tags else {
			return ContentRating::Safe;
		};
		let mut suggestive = false;
		for tag in tags {
			match tag.to_ascii_lowercase().as_str() {
				"18" | "18+" | "adult" | "hentai" | "pornographic" | "smut" => {
					return ContentRating::NSFW;
				}
				"ecchi" | "mature" | "soft yaoi" | "soft yuri" | "yaoi" | "yaoibl" | "yuri" => {
					suggestive = true;
				}
				_ => {}
			}
		}
		if suggestive {
			ContentRating::Suggestive
		} else {
			ContentRating::Safe
		}
	}

	fn get_dynamic_filters(&self, params: &Params) -> Result<Vec<Filter>> {
		let html = Request::get(format!("{}{}", params.base_url, params.genre_endpoint))?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let mut pairs: Vec<(String, String)> = Vec::new();
		for input in html.select("input[name]").into_iter().flatten() {
			if input.attr("name").as_deref() != Some("genre[]") {
				continue;
			}
			let Some(id) = input.attr("value") else {
				continue;
			};
			let Some(input_id) = input.attr("id") else {
				continue;
			};
			let Some(label) = html
				.select_first(format!("label[for='{input_id}']"))
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.map(|t| String::from(t.trim_end_matches([',', ' '])))
			else {
				continue;
			};
			if id.is_empty()
				|| !is_valid_genre(&label)
				|| pairs.iter().any(|(seen_opt, seen_id)| {
					seen_id == &id || seen_opt.eq_ignore_ascii_case(&label)
				}) {
				continue;
			}
			pairs.push((label, id));
		}
		pairs.sort_unstable_by(|a, b| a.0.cmp(&b.0));
		let (options, ids): (Vec<_>, Vec<_>) = pairs
			.into_iter()
			.map(|(opt, id)| (opt.into(), id.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genre[]".into(),
				title: Some("Genres".into()),
				is_genre: true,
				can_exclude: false,
				uses_tag_style: true,
				options,
				ids: Some(ids),
				..Default::default()
			}
			.into(),
		])
	}

	fn get_page_list(&self, params: &Params, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let url = format!("{}{}", params.base_url, chapter.key);
		let html = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let mut context = PageContext::new();
		context.insert("Referer".into(), url);
		let pages: Vec<Page> = html
			.select(
				".asura-stream-images img, .reading-content .page-break img, .reading-content img.wp-manga-chapter-img",
			)
			.into_iter()
			.flatten()
			.filter_map(|img| {
				let src = img.img_attr(false)?;
				(!src.is_empty()).then(|| Page {
					content: PageContent::url_context(src, context.clone()),
					..Default::default()
				})
			})
			.collect();
		if pages.is_empty() {
			bail!("No pages found");
		}
		Ok(pages)
	}

	fn get_manga_list(
		&self,
		_params: &Params,
		listing: Listing,
		page: i32,
	) -> Result<MangaPageResult> {
		let order = match listing.id.as_str() {
			"trending" | "latest" | "views" | "new-manga" | "rating" => listing.id.as_str(),
			_ => "latest",
		};
		let url = if page <= 1 {
			format!("{BASE_URL}/manga/?m_orderby={order}")
		} else {
			format!("{BASE_URL}/manga/page/{page}/?m_orderby={order}")
		};
		let html = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;
		let entries = html
			.select("article.asura-card, .c-tabs-item__content")
			.into_iter()
			.flatten()
			.filter_map(|card| card_manga(&card))
			.collect();
		let has_next_page = html
			.select_first(".asura-pagination a.next, a.next.page-numbers, a.nextpostslink")
			.is_some();
		Ok(MangaPageResult {
			entries,
			has_next_page,
		})
	}

	fn get_home(&self, _params: &Params) -> Result<HomeLayout> {
		let home = Request::get(format!("{BASE_URL}/"))?
			.header("Referer", &format!("{BASE_URL}/"))
			.html()?;

		let mut components = Vec::new();

		let featured = hero_entries(&home);
		if !featured.is_empty() {
			components.push(HomeComponent {
				title: Some("Trending".into()),
				subtitle: None,
				value: HomeComponentValue::BigScroller {
					entries: featured,
					auto_scroll_interval: Some(6.0),
				},
			});
		}

		let trending_now: Vec<Link> = swimlane_manga(&home, "Trending Now")
			.into_iter()
			.map(Into::into)
			.collect();
		if !trending_now.is_empty() {
			components.push(HomeComponent {
				title: Some("Trending Now".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: trending_now,
					listing: Some(listing("trending", "Trending Now")),
				},
			});
		}

		let latest = swimlane_latest(&home, "Latest Updates");
		if !latest.is_empty() {
			components.push(HomeComponent {
				title: Some("Latest Updates".into()),
				subtitle: None,
				value: HomeComponentValue::MangaChapterList {
					page_size: Some(5),
					entries: latest,
					listing: Some(listing("latest", "Latest Updates")),
				},
			});
		}

		let popular: Vec<Link> = swimlane_manga(&home, "Popular Manga")
			.into_iter()
			.map(Into::into)
			.collect();
		if !popular.is_empty() {
			components.push(HomeComponent {
				title: Some("Popular Manga".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: popular,
					listing: Some(listing("views", "Popular Manga")),
				},
			});
		}

		let new_releases: Vec<Link> = swimlane_manga(&home, "New Releases")
			.into_iter()
			.map(Into::into)
			.collect();
		if !new_releases.is_empty() {
			components.push(HomeComponent {
				title: Some("New Releases".into()),
				subtitle: None,
				value: HomeComponentValue::Scroller {
					entries: new_releases,
					listing: Some(listing("new-manga", "New Releases")),
				},
			});
		}

		Ok(HomeLayout { components })
	}
}

register_source!(
	Madara<Cocomic>,
	Home,
	ListingProvider,
	DynamicFilters,
	DeepLinkHandler,
	ImageRequestProvider
);
