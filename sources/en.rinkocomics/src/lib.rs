#![no_std]
mod chapters;
#[cfg(test)]
mod test;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, DynamicFilters, Filter, FilterValue,
	Home, HomeComponent, HomeComponentValue, HomeLayout, ImageRequestProvider, Link, Listing,
	ListingKind, ListingProvider, Manga, MangaPageResult, MangaStatus, MangaWithChapter,
	MultiSelectFilter, Page, PageContent, PageContext, Result, Source, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	helpers::{string::PlainText, uri::QueryParameters},
	imports::{
		defaults::defaults_get,
		html::{Document, Element, Html},
		net::Request,
		std::send_partial_result,
	},
	prelude::*,
};
use chapters::{
	LOCK_SUFFIX, chapter_number_of, collect_chapters, decode_cw_pages, has_more_chapters,
	is_locked_row, parse_rinko_date,
};

pub const BASE_URL: &str = "https://rinkocomics.com";
const SORTS: [&str; 4] = ["updated", "views", "new", "az"];

pub fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn abs_url(url: &str) -> String {
	let trimmed = url.trim();
	if trimmed.is_empty() {
		String::new()
	} else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
		trimmed.to_string()
	} else if trimmed.starts_with('/') {
		format!("{BASE_URL}{trimmed}")
	} else {
		format!("{BASE_URL}/{trimmed}")
	}
}

pub fn path_of(href: &str) -> String {
	let absolute = abs_url(href);
	let without_origin = absolute
		.strip_prefix(BASE_URL)
		.or_else(|| {
			absolute
				.strip_prefix("https://www.rinkocomics.com")
				.or_else(|| absolute.strip_prefix("http://rinkocomics.com"))
		})
		.unwrap_or(&absolute);
	without_origin
		.split(['?', '#'])
		.next()
		.unwrap_or("")
		.trim_matches('/')
		.to_string()
}

fn show_locked_chapters() -> bool {
	defaults_get::<bool>("showLocked").unwrap_or(false)
}

fn image_from(element: &Element, selector: &str) -> Option<String> {
	let img = element.select_first(selector)?;
	let src = img
		.attr("data-src")
		.or_else(|| img.attr("data-lazy-src"))
		.or_else(|| img.attr("src"))?;
	let cleaned = src.trim();
	(!cleaned.is_empty()).then(|| abs_url(cleaned))
}

fn parse_status(raw: &str) -> MangaStatus {
	match raw.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("completed") => MangaStatus::Completed,
		s if s.contains("hiatus") || s.contains("season") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("drop") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

fn make_listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}

fn parse_editors_choice(home: &Document) -> Vec<Manga> {
	let mut seen: Vec<String> = Vec::new();
	home.select("[data-cw-slider] [data-slide]")
		.map(|slides| {
			slides
				.filter_map(|slide| {
					let href = slide.select_first("a.btn-primary")?.attr("href")?;
					let key = path_of(&href);
					if key.is_empty() || seen.contains(&key) {
						return None;
					}
					let title = slide
						.select_first("h2")
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					seen.push(key.clone());
					let cover = image_from(&slide, "img");
					let description = slide
						.select_first("p")
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty());
					let mut tags: Vec<String> = Vec::new();
					if let Some(spans) = slide.select("div.flex-wrap > span") {
						for span in spans {
							if let Some(text) = span.text() {
								for part in text.split('·') {
									let tag = clean(part);
									if !tag.is_empty() && !tags.contains(&tag) {
										tags.push(tag);
									}
								}
							}
						}
					}
					Some(Manga {
						url: Some(format!("{BASE_URL}/{key}/")),
						key,
						title,
						cover,
						description,
						tags: (!tags.is_empty()).then_some(tags),
						content_rating: ContentRating::Safe,
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

fn parse_featured_week(home: &Document) -> Vec<Manga> {
	let mut seen: Vec<String> = Vec::new();
	home.select("section[aria-labelledby='cw-week-m'] a")
		.map(|cards| {
			cards
				.filter_map(|card| {
					let href = card.attr("href")?;
					let key = path_of(&href);
					if key.is_empty() || seen.contains(&key) {
						return None;
					}
					let title = card
						.select_first(".line-clamp-2")
						.and_then(|el| el.text())
						.or_else(|| card.attr("title"))
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					seen.push(key.clone());
					Some(Manga {
						url: Some(format!("{BASE_URL}/{key}/")),
						key,
						title,
						cover: image_from(&card, "img"),
						content_rating: ContentRating::Safe,
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

fn parse_new_novels(home: &Document) -> Vec<Manga> {
	let mut seen: Vec<String> = Vec::new();
	home.select("section[aria-labelledby='cw-novels'] a[href*='/novel/']")
		.map(|cards| {
			cards
				.filter_map(|card| {
					let href = card.attr("href")?;
					let key = path_of(&href);
					if key.is_empty() || seen.contains(&key) {
						return None;
					}
					let title = card
						.select_first(".line-clamp-2")
						.and_then(|el| el.text())
						.or_else(|| card.attr("title"))
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					seen.push(key.clone());
					Some(Manga {
						url: Some(format!("{BASE_URL}/{key}/")),
						key,
						title,
						cover: image_from(&card, "img"),
						content_rating: ContentRating::Safe,
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

fn parse_latest_releases(doc: &Document, show_locked: bool) -> Vec<MangaWithChapter> {
	let mut seen: Vec<String> = Vec::new();
	doc.select("#cw-latest article")
		.map(|articles| {
			articles
				.filter_map(|article| {
					let link = article.select_first("a.line-clamp-2")?;
					let href = link.attr("href")?;
					let key = path_of(&href);
					if key.is_empty() || seen.contains(&key) {
						return None;
					}
					let title = link.text().map(|t| clean(&t)).filter(|t| !t.is_empty())?;

					let rows: Vec<Element> = article
						.select("a.chapter-item")
						.map(|els| els.collect())
						.unwrap_or_default();
					let picked = rows
						.iter()
						.find(|row| {
							let ch_href = row.attr("href").unwrap_or_default();
							!is_locked_row(row, &ch_href)
								&& (ch_href.contains("/chapter/")
									|| ch_href.contains("/novel_chapter/"))
						})
						.or_else(|| show_locked.then(|| rows.first()).flatten())?;

					let ch_href = picked.attr("href").unwrap_or_default();
					let locked = is_locked_row(picked, &ch_href);
					let ch_path = if !ch_href.is_empty() && ch_href != "#" {
						path_of(&ch_href)
					} else {
						format!("locked-{key}")
					};
					let ch_key = if locked {
						format!("{ch_path}{LOCK_SUFFIX}")
					} else {
						ch_path.clone()
					};
					let spans: Vec<Element> = picked
						.select("span")
						.map(|els| els.collect())
						.unwrap_or_default();
					let label = spans
						.first()
						.and_then(|el| el.text())
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					let stamp = spans.last().and_then(|el| el.text()).map(|t| clean(&t));
					seen.push(key.clone());

					let chapter_number = chapter_number_of(&label);
					Some(MangaWithChapter {
						manga: Manga {
							url: Some(format!("{BASE_URL}/{key}/")),
							key,
							title,
							cover: image_from(&article, "img"),
							content_rating: ContentRating::Safe,
							..Default::default()
						},
						chapter: Chapter {
							key: ch_key,
							title: Some(label),
							chapter_number,
							date_uploaded: stamp.as_deref().and_then(parse_rinko_date),
							locked,
							url: (!ch_path.starts_with("locked-"))
								.then(|| format!("{BASE_URL}/{ch_path}/")),
							..Default::default()
						},
					})
				})
				.collect()
		})
		.unwrap_or_default()
}

fn parse_browse_page(doc: &Document) -> MangaPageResult {
	let mut seen: Vec<String> = Vec::new();
	let entries = doc
		.select("#cwBrowse ul > li")
		.map(|items| {
			items
				.filter_map(|item| {
					let link = item.select_first("a")?;
					let href = link.attr("href")?;
					let key = path_of(&href);
					if key.is_empty() || seen.contains(&key) {
						return None;
					}
					let title = item
						.select_first(".line-clamp-2")
						.and_then(|el| el.text())
						.or_else(|| link.attr("title"))
						.map(|t| clean(&t))
						.filter(|t| !t.is_empty())?;
					seen.push(key.clone());
					let status = item
						.select("span.text-subtle")
						.and_then(|mut els| els.next_back())
						.and_then(|el| el.text())
						.map(|t| parse_status(&t))
						.unwrap_or(MangaStatus::Unknown);
					Some(Manga {
						url: Some(format!("{BASE_URL}/{key}/")),
						key,
						title,
						cover: image_from(&item, "img"),
						status,
						content_rating: ContentRating::Safe,
						..Default::default()
					})
				})
				.collect()
		})
		.unwrap_or_default();
	let has_next_page = doc
		.select_first("#cwBrowse a.next, #cwBrowse a[rel='next']")
		.is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

fn build_home_layout(
	home: &Document,
	new_comics: &Document,
	novel_home: &Document,
	show_locked: bool,
) -> HomeLayout {
	let mut components: Vec<HomeComponent> = Vec::new();

	let editors_choice = parse_editors_choice(home);
	if !editors_choice.is_empty() {
		components.push(HomeComponent {
			title: Some("Editor's Choice".into()),
			subtitle: None,
			value: HomeComponentValue::BigScroller {
				entries: editors_choice,
				auto_scroll_interval: Some(6.0),
			},
		});
	}

	let featured_week: Vec<Link> = parse_featured_week(home)
		.into_iter()
		.map(Into::into)
		.collect();
	if !featured_week.is_empty() {
		components.push(HomeComponent {
			title: Some("Featured This Week".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: featured_week,
				listing: Some(make_listing("views", "Featured This Week")),
			},
		});
	}

	let latest_comics = parse_latest_releases(home, show_locked);
	if !latest_comics.is_empty() {
		components.push(HomeComponent {
			title: Some("Latest Releases (Comics)".into()),
			subtitle: None,
			value: HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: latest_comics,
				listing: Some(make_listing("updated", "Latest Releases (Comics)")),
			},
		});
	}

	let new_comics_links: Vec<Link> = parse_browse_page(new_comics)
		.entries
		.into_iter()
		.map(Into::into)
		.collect();
	if !new_comics_links.is_empty() {
		components.push(HomeComponent {
			title: Some("New Comics".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: new_comics_links,
				listing: Some(make_listing("new", "New Comics")),
			},
		});
	}

	let new_novels: Vec<Link> = parse_new_novels(home).into_iter().map(Into::into).collect();
	if !new_novels.is_empty() {
		components.push(HomeComponent {
			title: Some("New Novels".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: new_novels,
				listing: Some(make_listing("novels", "New Novels")),
			},
		});
	}

	let latest_novels = parse_latest_releases(novel_home, show_locked);
	if !latest_novels.is_empty() {
		components.push(HomeComponent {
			title: Some("Latest Releases (Novels)".into()),
			subtitle: None,
			value: HomeComponentValue::MangaChapterList {
				page_size: Some(5),
				entries: latest_novels,
				listing: Some(make_listing("novels-updated", "Latest Releases (Novels)")),
			},
		});
	}

	HomeLayout { components }
}

fn fetch_html(url: &str) -> Result<Document> {
	Ok(Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()?)
}

struct RinkoComics;

impl Source for RinkoComics {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let mut kind = String::from("comic");
		let mut qs = QueryParameters::new();
		if let Some(q) = query.as_deref().map(str::trim).filter(|q| !q.is_empty()) {
			qs.push("q", Some(q));
		}
		for filter in filters {
			match filter {
				FilterValue::Sort { index, .. } => {
					if let Some(sort) = SORTS.get(index as usize) {
						qs.set("sort", Some(sort));
					}
				}
				FilterValue::Select { id, value } if !value.is_empty() => match id.as_str() {
					"type" => kind = value,
					"genre" | "genres[]" => qs.push("genres[]", Some(&value)),
					_ => qs.set(&id, Some(&value)),
				},
				FilterValue::MultiSelect { id, included, .. } if id == "genres[]" => {
					for genre in included {
						qs.push("genres[]", Some(&genre));
					}
				}
				_ => {}
			}
		}
		let path = if page <= 1 {
			format!("/{kind}/")
		} else {
			format!("/{kind}/page/{page}/")
		};
		// path is built from /{kind}/ or /{kind}/page/{page}/, so it has no query yet.
		let url = if qs.is_empty() {
			format!("{BASE_URL}{path}")
		} else {
			format!("{BASE_URL}{path}?{qs}")
		};
		Ok(parse_browse_page(&fetch_html(&url)?))
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		let key = manga.key.trim_matches('/').to_string();
		let url = format!("{BASE_URL}/{key}/");
		let body = Request::get(&url)?
			.header("Referer", &format!("{BASE_URL}/"))
			.string()?;
		let html = Html::parse_with_url(&body, &url)?;
		let is_novel = key.starts_with("novel/");

		if needs_details {
			if let Some(title) = html
				.select_first(".comic-info-upper h1, h1.novel-title, main h1, h1")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
			{
				manga.title = title;
			}
			manga.cover = html
				.select_first("meta[property='og:image']")
				.and_then(|el| el.attr("content"))
				.map(|s| abs_url(&s))
				.filter(|s| !s.is_empty())
				.or_else(|| image_from(&html.select_first("main")?, "aside img, img"))
				.or_else(|| manga.cover.take());
			manga.description = html
				.select_first("#cwSynopsis, .comic-synopsis, .novel-synopsis")
				.and_then(|el| el.text())
				.or_else(|| {
					html.select_first("meta[property='og:description']")
						.and_then(|el| el.attr("content"))
				})
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty());

			let mut authors: Vec<String> = Vec::new();
			if let Some(spans) = html.select(".comic-graph > span") {
				for span in spans {
					if let Some(name) = span.text().map(|t| clean(&t))
						&& !name.is_empty()
						&& name != "•" && !authors.contains(&name)
					{
						authors.push(name);
					}
				}
			}
			manga.authors = (!authors.is_empty()).then_some(authors);

			let mut status = MangaStatus::Unknown;
			let mut series_type = String::new();
			if let Some(dts) = html.select("#main dt, main dt") {
				for dt in dts {
					let Some(label) = dt.text().map(|t| clean(&t)) else {
						continue;
					};
					let Some(value) = dt.next().and_then(|dd| dd.text()).map(|t| clean(&t)) else {
						continue;
					};
					if label.eq_ignore_ascii_case("status") && status == MangaStatus::Unknown {
						status = parse_status(&value);
					} else if label.eq_ignore_ascii_case("type") && series_type.is_empty() {
						series_type = value;
					}
				}
			}
			manga.status = status;

			let mut tags: Vec<String> = Vec::new();
			if let Some(links) =
				html.select("#main a[href*='/comics_genres/'], main a[href*='/comics_genres/']")
			{
				for link in links {
					if let Some(tag) = link.text().map(|t| clean(&t)).filter(|t| !t.is_empty())
						&& !tags.iter().any(|t| t.eq_ignore_ascii_case(&tag))
					{
						tags.push(tag);
					}
				}
			}
			manga.viewer = if is_novel {
				Viewer::Unknown
			} else if series_type.eq_ignore_ascii_case("manga") {
				Viewer::RightToLeft
			} else {
				Viewer::Webtoon
			};
			manga.content_rating = ContentRating::Safe;
			manga.tags = (!tags.is_empty()).then_some(tags);
			manga.url = Some(url.clone());

			if needs_chapters && has_more_chapters(&html) {
				send_partial_result(&manga);
			}
		}

		if needs_chapters {
			manga.chapters = Some(collect_chapters(&html, &body, &url, show_locked_chapters()));
		}

		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		if chapter.locked || chapter.key.ends_with(LOCK_SUFFIX) {
			bail!("This chapter is locked on RinkoComics");
		}
		let path = chapter.key.trim_matches('/');
		let url = format!("{BASE_URL}/{path}/");
		let html = fetch_html(&url)?;

		if let Some(prose) = html.select_first("#cwNovelText, #textContent, .novel-content") {
			let mut paragraphs: Vec<String> = Vec::new();
			if let Some(nodes) = prose.select("p, h2, h3, h4, blockquote") {
				for node in nodes {
					if let Some(text) = node.text().map(|t| clean(&t)).filter(|t| !t.is_empty()) {
						paragraphs.push(text.escape_markdown());
					}
				}
			}
			let text = if !paragraphs.is_empty() {
				paragraphs.join("\n\n")
			} else {
				prose
					.text()
					.map(|t| clean(&t).escape_markdown())
					.unwrap_or_default()
			};
			if text.is_empty() {
				bail!("No novel text found for this episode");
			}
			return Ok(vec![Page {
				content: PageContent::text(text),
				..Default::default()
			}]);
		}

		let mut context = PageContext::new();
		context.insert("Referer".into(), url);

		if let Some(encoded) = html
			.select_first("#cwPages")
			.and_then(|el| el.text())
			.filter(|t| !t.is_empty())
		{
			let pages = decode_cw_pages(&encoded)?;
			if !pages.is_empty() {
				return Ok(pages
					.into_iter()
					.map(|img_url| Page {
						content: PageContent::url_context(img_url, context.clone()),
						..Default::default()
					})
					.collect());
			}
		}

		let pages: Vec<Page> = html
			.select("img.chapter-image")
			.map(|imgs| {
				imgs.filter_map(|img| {
					let src = img
						.attr("data-src")
						.or_else(|| img.attr("data-lazy-src"))
						.or_else(|| img.attr("src"))?;
					let cleaned = src.trim();
					(!cleaned.is_empty()).then(|| Page {
						content: PageContent::url_context(abs_url(cleaned), context.clone()),
						..Default::default()
					})
				})
				.collect()
			})
			.unwrap_or_default();
		if pages.is_empty() {
			bail!("No pages found for this chapter");
		}
		Ok(pages)
	}
}

impl DynamicFilters for RinkoComics {
	fn get_dynamic_filters(&self) -> Result<Vec<Filter>> {
		let html = fetch_html(&format!("{BASE_URL}/comic/"))?;
		let mut pairs: Vec<(String, String)> = Vec::new();
		if let Some(inputs) = html.select("#cwBrowse input[name]") {
			for input in inputs {
				if input.attr("name").as_deref() != Some("genres[]") {
					continue;
				}
				let Some(id) = input
					.attr("value")
					.map(|s| clean(&s))
					.filter(|s| !s.is_empty())
				else {
					continue;
				};
				let Some(label) = input
					.parent()
					.and_then(|p| p.select_first("span"))
					.and_then(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty())
				else {
					continue;
				};
				if !pairs.iter().any(|(_, seen_id)| seen_id == &id) {
					pairs.push((label, id));
				}
			}
		}
		let (options, ids): (Vec<_>, Vec<_>) = pairs
			.into_iter()
			.map(|(opt, id)| (opt.into(), id.into()))
			.unzip();
		Ok(vec![
			MultiSelectFilter {
				id: "genres[]".into(),
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
}

impl ListingProvider for RinkoComics {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		let (kind, sort) = match listing.id.as_str() {
			"views" => ("comic", "views"),
			"new" => ("comic", "new"),
			"updated" => ("comic", "updated"),
			"novels" => ("novel", "new"),
			"novels-updated" => ("novel", "updated"),
			_ => ("comic", "updated"),
		};
		let path = if page <= 1 {
			format!("/{kind}/")
		} else {
			format!("/{kind}/page/{page}/")
		};
		Ok(parse_browse_page(&fetch_html(&format!(
			"{BASE_URL}{path}?sort={sort}"
		))?))
	}
}

impl Home for RinkoComics {
	fn get_home(&self) -> Result<HomeLayout> {
		let requests = [
			Request::get(format!("{BASE_URL}/"))?.header("Referer", &format!("{BASE_URL}/")),
			Request::get(format!("{BASE_URL}/comic/?sort=new"))?
				.header("Referer", &format!("{BASE_URL}/")),
			Request::get(format!("{BASE_URL}/?rel=novels"))?
				.header("Referer", &format!("{BASE_URL}/")),
		];
		let [home_resp, new_comics_resp, novel_home_resp]: [_; 3] = Request::send_all(requests)
			.try_into()
			.map_err(|_| error!("Unexpected response count"))?;
		let home = home_resp?.get_html()?;
		let new_comics = new_comics_resp?.get_html()?;
		let novel_home = novel_home_resp?.get_html()?;
		Ok(build_home_layout(
			&home,
			&new_comics,
			&novel_home,
			show_locked_chapters(),
		))
	}
}

impl ImageRequestProvider for RinkoComics {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?.header("Referer", &format!("{BASE_URL}/")))
	}
}

impl DeepLinkHandler for RinkoComics {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let path = path_of(&url);
		if path.starts_with("comic/") || path.starts_with("novel/") {
			return Ok(Some(DeepLinkResult::Manga { key: path }));
		}
		if path.starts_with("chapter/") || path.starts_with("novel_chapter/") {
			let html = fetch_html(&url)?;
			let selector = if path.starts_with("novel_chapter/") {
				"main a[href*='/novel/'], a[href*='/novel/']"
			} else {
				"main a[href*='/comic/'], a[href*='/comic/']"
			};
			if let Some(manga_href) = html.select_first(selector).and_then(|a| a.attr("href")) {
				let manga_key = path_of(&manga_href);
				if !manga_key.is_empty() {
					return Ok(Some(DeepLinkResult::Chapter {
						manga_key,
						key: path,
					}));
				}
			}
		}
		Ok(None)
	}
}

register_source!(
	RinkoComics,
	Home,
	ListingProvider,
	DynamicFilters,
	ImageRequestProvider,
	DeepLinkHandler
);
