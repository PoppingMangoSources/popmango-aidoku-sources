#![no_std]
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent, HomeComponentValue,
	HomeLayout, Link, Manga, MangaPageResult, Page, PageContent, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{string::PlainText, uri::encode_uri_component},
	imports::std::current_date,
	prelude::*,
};

mod helpers;
mod models;
#[cfg(test)]
mod test;

use helpers::{locked_chapters, next_data, paragraphs};
use models::{Block, BrowseProps, ChapterProps, Entry, HomeProps, SeriesProps, Slide};

const BASE_URL: &str = "https://flamecomics.xyz";
const CDN_URL: &str = "https://cdn.flamecomics.xyz";

struct FlameComics;

impl Source for FlameComics {
	fn new() -> Self {
		Self
	}

	// Browse lists the whole catalogue in one page, which the site also filters locally.
	fn get_search_manga_list(
		&self,
		query: Option<String>,
		_page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut series = next_data::<BrowseProps>("/browse")?.series;
		let query = query.map(|query| query.to_lowercase());
		let (mut popular, mut ascending) = (false, true);
		let (mut genres, mut hidden_genres) = (Vec::new(), Vec::new());
		let (mut types, mut statuses, mut years) = (Vec::new(), Vec::new(), Vec::new());
		for filter in filters {
			match filter {
				FilterValue::Sort {
					index,
					ascending: order,
					..
				} => {
					popular = index == 1;
					ascending = order;
				}
				FilterValue::MultiSelect {
					id,
					included,
					excluded,
				} => match id.as_str() {
					"genre" => {
						genres = included;
						hidden_genres = excluded;
					}
					"type" => types = included,
					"status" => statuses = included,
					"year" => years = included,
					_ => {}
				},
				_ => {}
			}
		}
		series.retain(|entry| {
			query
				.as_deref()
				.is_none_or(|query| entry.title.to_lowercase().contains(query))
				&& genres.iter().all(|genre| entry.tags.contains(genre))
				&& !hidden_genres.iter().any(|genre| entry.tags.contains(genre))
				&& (types.is_empty() || entry.r#type.as_ref().is_some_and(|t| types.contains(t)))
				&& (statuses.is_empty()
					|| entry.status.as_deref().is_some_and(|status| {
						statuses.iter().any(|s| s.eq_ignore_ascii_case(status))
					})) && (years.is_empty()
				|| entry
					.year
					.is_some_and(|year| years.contains(&year.to_string())))
		});
		series.sort_by(|a, b| {
			let title = a.title.to_lowercase().cmp(&b.title.to_lowercase());
			let order = if popular {
				let rank = |entry: &Entry| entry.popularity_rank.unwrap_or(u32::MAX);
				rank(a).cmp(&rank(b)).then(title)
			} else {
				title
			};
			if ascending { order } else { order.reverse() }
		});
		Ok(MangaPageResult {
			entries: series.into_iter().map(Entry::into_card).collect(),
			has_next_page: false,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let SeriesProps { series, chapters } = next_data(&manga.key)?;
		if needs_chapters {
			let locks = locked_chapters(&series.unlock, &chapters, current_date());
			let url = format!("{BASE_URL}{}", manga.key);
			manga.chapters = Some(
				chapters
					.into_iter()
					.zip(locks)
					.map(|(chapter, locked)| chapter.into_chapter(&url, locked))
					.collect(),
			);
		}
		if needs_details {
			manga.copy_from(series.into());
		}
		Ok(manga)
	}

	fn get_page_list(&self, manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let reader = next_data::<ChapterProps>(&format!("{}{}", manga.key, chapter.key))?.chapter;
		if manga.key.starts_with("/novel/") {
			let Some(content) = reader.content else {
				bail!("This chapter is in early access on Flame Comics");
			};
			let text = paragraphs(&content)
				.iter()
				.map(|paragraph| paragraph.escape_markdown())
				.collect::<Vec<_>>()
				.join("\n\n");
			if text.is_empty() {
				bail!("This chapter has no text");
			}
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}
		let series_id = reader
			.series_id
			.ok_or_else(|| error!("This chapter has no series"))?;
		let pages: Vec<Page> = reader
			.images
			.into_values()
			.map(|image| Page {
				// The site's reader adds the edit time, so re-uploaded pages aren't cached.
				content: PageContent::url(format!(
					"{CDN_URL}/uploads/images/series/{series_id}/{}/{}?{}",
					reader.token,
					encode_uri_component(image.name),
					reader.edit_time.unwrap_or_default()
				)),
				..Default::default()
			})
			.collect();
		if pages.is_empty() {
			bail!("This chapter has no pages");
		}
		Ok(pages)
	}
}

impl Home for FlameComics {
	// Sections follow the site's home page, under its banner carousel.
	fn get_home(&self) -> Result<HomeLayout> {
		let home = next_data::<HomeProps>("/")?;
		let mut components = Vec::new();

		let banners: Vec<Link> = home
			.carousel
			.into_iter()
			.filter_map(Slide::into_link)
			.collect();
		if !banners.is_empty() {
			components.push(HomeComponent {
				title: Some("Featured".into()),
				subtitle: None,
				// The banners are about 8:3; a fixed size smooths preloading.
				value: HomeComponentValue::ImageScroller {
					links: banners,
					auto_scroll_interval: Some(6.0),
					width: Some(400),
					height: Some(150),
				},
			});
		}

		let scroller = |block: Block| HomeComponent {
			title: Some(block.title),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: block
					.series
					.into_iter()
					.map(|entry| entry.into_card().into())
					.collect(),
				listing: None,
			},
		};
		let latest = |block: Block| HomeComponent {
			title: Some(block.title),
			subtitle: None,
			value: HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: block
					.series
					.into_iter()
					.filter_map(Entry::into_latest)
					.collect(),
				listing: None,
			},
		};
		components.extend(home.novels.into_block().map(scroller));
		components.extend(home.popular.into_block().map(scroller));
		components.extend(home.staff_picks.into_block().map(scroller));
		components.extend(home.latest.into_block().map(latest));
		components.extend(home.latest_novels.into_block().map(latest));
		Ok(HomeLayout { components })
	}
}

impl DeepLinkHandler for FlameComics {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(path) = url.strip_prefix(BASE_URL) else {
			return Ok(None);
		};
		let path = path.split(['?', '#']).next().unwrap_or_default();
		let segments: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
		Ok(match segments.as_slice() {
			[kind @ ("series" | "novel"), id] => Some(DeepLinkResult::Manga {
				key: format!("/{kind}/{id}"),
			}),
			[kind @ ("series" | "novel"), id, token] => Some(DeepLinkResult::Chapter {
				manga_key: format!("/{kind}/{id}"),
				key: format!("/{token}"),
			}),
			_ => None,
		})
	}
}

register_source!(FlameComics, Home, DeepLinkHandler);
