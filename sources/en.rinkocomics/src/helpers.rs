use crate::chapters::{
	LOCK_SUFFIX, chapter_number_of, clean_chapter_title, is_locked_row, parse_date_str,
};
use aidoku::{
	Chapter, ContentRating, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	ListingKind, Manga, MangaPageResult, MangaStatus, MangaWithChapter, Result,
	alloc::{String, Vec, collections::BTreeSet, format, string::ToString},
	imports::{
		html::{Document, Element},
		net::Request,
	},
};

pub const BASE_URL: &str = "https://rinkocomics.com";
pub const SORTS: [&str; 4] = ["az", "updated", "views", "new"];

pub fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn abs_url(url: &str) -> String {
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
	let without_origin = absolute.strip_prefix(BASE_URL).unwrap_or(&absolute);
	without_origin
		.split(['?', '#'])
		.next()
		.unwrap_or("")
		.trim_matches('/')
		.into()
}

fn largest_srcset(srcset: &str) -> Option<&str> {
	srcset
		.split(',')
		.filter_map(|entry| {
			let mut parts = entry.split_whitespace();
			let url = parts.next()?;
			let width = parts
				.next()
				.and_then(|w| w.strip_suffix('w'))
				.and_then(|w| w.parse::<u32>().ok())
				.unwrap_or(0);
			Some((url, width))
		})
		.max_by_key(|(_, width)| *width)
		.map(|(url, _)| url)
}

pub fn image_from(element: &Element, selector: &str) -> Option<String> {
	let img = element.select_first(selector)?;
	let from_srcset = img
		.attr("data-srcset")
		.or_else(|| img.attr("srcset"))
		.and_then(|s| largest_srcset(&s).map(String::from));
	let src = from_srcset
		.or_else(|| img.attr("data-src"))
		.or_else(|| img.attr("data-lazy-src"))
		.or_else(|| img.attr("src"))?;
	let cleaned = src.trim();
	(!cleaned.is_empty()).then(|| abs_url(cleaned))
}

pub fn parse_status(raw: &str) -> MangaStatus {
	match raw.trim().to_ascii_lowercase().as_str() {
		s if s.contains("ongoing") => MangaStatus::Ongoing,
		s if s.contains("completed") => MangaStatus::Completed,
		s if s.contains("hiatus") || s.contains("season") => MangaStatus::Hiatus,
		s if s.contains("cancel") || s.contains("drop") => MangaStatus::Cancelled,
		_ => MangaStatus::Unknown,
	}
}

pub fn fetch_html(url: &str) -> Result<Document> {
	Ok(Request::get(url)?
		.header("Referer", &format!("{BASE_URL}/"))
		.html()?)
}

pub fn listing(id: &str, name: &str) -> Listing {
	Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	}
}

fn parse_editors_choice(home: &Document) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	home.select("[data-cw-slider] [data-slide]")
		.into_iter()
		.flatten()
		.filter_map(|slide| {
			let href = slide.select_first("a.btn-primary")?.attr("href")?;
			let key = path_of(&href);
			if key.is_empty() || !seen.insert(key.clone()) {
				return None;
			}
			let title = slide
				.select_first("h2")
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			// Desktop slides mark genre pills with `span.chip`, excluding the chapter-count badge.
			let mut tags: Vec<String> = slide
				.select("span.chip")
				.into_iter()
				.flatten()
				.filter_map(|span| span.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())
				.collect();
			tags.sort_unstable();
			tags.dedup();
			Some(Manga {
				url: Some(format!("{BASE_URL}/{key}/")),
				key,
				title,
				cover: image_from(&slide, "img"),
				description: slide
					.select_first("p")
					.and_then(|el| el.text())
					.map(|t| clean(&t))
					.filter(|t| !t.is_empty()),
				tags: (!tags.is_empty()).then_some(tags),
				content_rating: ContentRating::Safe,
				..Default::default()
			})
		})
		.collect()
}

fn parse_simple_cards(doc: &Document, selector: &str) -> Vec<Manga> {
	let mut seen = BTreeSet::new();
	doc.select(selector)
		.into_iter()
		.flatten()
		.filter_map(|card| {
			let key = path_of(&card.attr("href")?);
			if key.is_empty() || !seen.insert(key.clone()) {
				return None;
			}
			let title = card
				.select_first(".line-clamp-2")
				.and_then(|el| el.text())
				.or_else(|| card.attr("title"))
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
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
}

fn parse_latest_releases(doc: &Document, show_locked: bool) -> Vec<MangaWithChapter> {
	let mut seen = BTreeSet::new();
	doc.select("#cw-latest article")
		.into_iter()
		.flatten()
		.filter_map(|article| {
			let link = article.select_first("a.line-clamp-2")?;
			let key = path_of(&link.attr("href")?);
			if key.is_empty() || !seen.insert(key.clone()) {
				return None;
			}
			let title = link.text().map(|t| clean(&t)).filter(|t| !t.is_empty())?;

			let rows: Vec<Element> = article
				.select("a.chapter-item")
				.into_iter()
				.flatten()
				.collect();
			let picked = rows
				.iter()
				.find(|row| {
					let ch_href = row.attr("href").unwrap_or_default();
					!is_locked_row(row, &ch_href)
						&& (ch_href.contains("/chapter/") || ch_href.contains("/novel_chapter/"))
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
				ch_path
			};
			let spans: Vec<Element> = picked.select("span").into_iter().flatten().collect();
			let label = spans
				.first()
				.and_then(|el| el.text())
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
			let stamp = spans.last().and_then(|el| el.text()).map(|t| clean(&t));
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
					title: clean_chapter_title(&label, chapter_number),
					chapter_number,
					date_uploaded: stamp.as_deref().and_then(parse_date_str),
					locked,
					..Default::default()
				},
			})
		})
		.collect()
}

pub fn parse_browse_page(doc: &Document) -> MangaPageResult {
	let mut seen = BTreeSet::new();
	let entries = doc
		.select("#cwBrowse ul > li")
		.into_iter()
		.flatten()
		.filter_map(|item| {
			let link = item.select_first("a")?;
			let key = path_of(&link.attr("href")?);
			if key.is_empty() || !seen.insert(key.clone()) {
				return None;
			}
			let title = item
				.select_first(".line-clamp-2")
				.and_then(|el| el.text())
				.or_else(|| link.attr("title"))
				.map(|t| clean(&t))
				.filter(|t| !t.is_empty())?;
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
		.collect();
	let has_next_page = doc
		.select_first("#cwBrowse a.next, #cwBrowse a[rel='next']")
		.is_some();
	MangaPageResult {
		entries,
		has_next_page,
	}
}

pub fn build_home_layout(
	home: &Document,
	new_comics: &Document,
	novel_home: &Document,
	show_locked: bool,
) -> HomeLayout {
	let mut components = Vec::new();

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

	let featured_week: Vec<Link> =
		parse_simple_cards(home, "section[aria-labelledby='cw-week-m'] a")
			.into_iter()
			.map(Into::into)
			.collect();
	if !featured_week.is_empty() {
		components.push(HomeComponent {
			title: Some("Featured This Week".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: featured_week,
				listing: Some(listing("views", "Featured This Week")),
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
				listing: Some(listing("updated", "Latest Releases (Comics)")),
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
				listing: Some(listing("new", "New Comics")),
			},
		});
	}

	let new_novels: Vec<Link> = parse_simple_cards(
		home,
		"section[aria-labelledby='cw-novels'] a[href*='/novel/']",
	)
	.into_iter()
	.map(Into::into)
	.collect();
	if !new_novels.is_empty() {
		components.push(HomeComponent {
			title: Some("New Novels".into()),
			subtitle: None,
			value: HomeComponentValue::Scroller {
				entries: new_novels,
				listing: Some(listing("novels", "New Novels")),
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
				listing: Some(listing("novels-updated", "Latest Releases (Novels)")),
			},
		});
	}

	HomeLayout { components }
}
