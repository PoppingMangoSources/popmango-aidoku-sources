use crate::{
	Impl, Params, helpers,
	models::{Post, SearchResponse},
};
use aidoku::{
	HomeComponent, HomeComponentValue, HomeLayout, Link, LinkValue, Manga, Result,
	alloc::{String, Vec, string::ToString, vec},
	helpers::uri::QueryParameters,
	imports::{
		html::{Document, Element},
		net::Request,
	},
	prelude::*,
};

pub fn get<T: Impl + ?Sized>(source: &T, params: &Params) -> Result<HomeLayout> {
	let mut urls = vec![params.base_url.to_string()];
	for (sort, series_type) in [("createdAt", None), (params.sorts[0], Some("NOVEL"))] {
		let mut query = QueryParameters::new();
		query.push("page", Some("1"));
		query.push("perPage", Some("18"));
		query.push("view", Some("archive"));
		query.push("orderBy", Some(sort));
		query.push("orderDirection", Some("desc"));
		if let Some(series_type) = series_type {
			query.push("seriesType", Some(series_type));
		}
		urls.push(format!("{}/api/query?{query}", params.api_url()));
	}
	// Pinned titles can fall below the first API page.
	urls.push(format!("{}/api/posts?page=1&perPage=100", params.api_url()));
	let requests = urls
		.iter()
		.map(|url| {
			Ok(Request::get(url)?
				.header(
					"Accept",
					if url == &params.base_url {
						"text/html"
					} else {
						"application/json"
					},
				)
				.header("Referer", &format!("{}/", params.base_url))
				.timeout(30.))
		})
		.collect::<Result<Vec<_>>>()?;
	let mut layout = HomeLayout::default();
	let mut failure = None;
	for (index, response) in Request::send_all(requests).into_iter().enumerate() {
		let result = (|| -> Result<()> {
			let mut response = helpers::check_response(response?, &urls[index])?;
			if index == 0 {
				layout = source.parse_home(params, &response.get_html()?)?;
			} else {
				let data: SearchResponse = response.get_json()?;
				if index + 1 == urls.len() {
					latest(&mut layout, data.posts, params)?;
					return Ok(());
				}
				let entries: Vec<Link> = data
					.posts
					.iter()
					.filter_map(|post| Manga::try_from(post).ok())
					.map(Link::from)
					.collect();
				if !entries.is_empty() {
					let component = HomeComponent {
						title: Some(
							if index == 1 {
								params.new_releases_title
							} else {
								params.novels_title
							}
							.into(),
						),
						value: HomeComponentValue::Scroller {
							entries,
							listing: None,
						},
						..Default::default()
					};
					if let Some(existing) = layout
						.components
						.iter_mut()
						.find(|c| c.title == component.title)
					{
						*existing = component;
						return Ok(());
					}
					let index = layout
						.components
						.iter()
						.position(|c| {
							matches!(c.title.as_deref(), Some("Most Popular" | "Tendencias"))
						})
						.unwrap_or(layout.components.len());
					layout.components.insert(index, component);
				}
			}
			Ok(())
		})();
		if let Err(error) = result {
			println!("Home request {} failed: {error:?}", urls[index]);
			failure = Some(error);
		}
	}
	if layout.components.is_empty() {
		return Err(failure.unwrap_or_else(|| error!("Home sections not found")));
	}
	Ok(layout)
}

pub(crate) fn latest(layout: &mut HomeLayout, mut posts: Vec<Post>, params: &Params) -> Result<()> {
	let Some(component) = layout.components.iter_mut().find(|component| {
		matches!(
			component.title.as_deref(),
			Some("Latest Releases" | "Latest Updates" | "Últimas Actualizaciones")
		)
	}) else {
		return Ok(());
	};
	let HomeComponentValue::MangaList { entries, .. } = &component.value else {
		return Ok(());
	};
	let mut chapters = Vec::new();
	for link in entries {
		let Some(LinkValue::Manga(manga)) = &link.value else {
			continue;
		};
		let slug = manga.key.strip_prefix("/series/").unwrap_or(&manga.key);
		let index = posts
			.iter()
			.position(|post| post.slug == slug)
			.ok_or_else(|| error!("Latest series missing from API: {slug}"))?;
		let post = posts.remove(index);
		if let Some(entry) = post.into_manga_with_chapter(&params.base_url)? {
			chapters.push(entry);
		}
	}
	if chapters.is_empty() {
		bail!("No accessible latest chapters");
	}
	component.value = HomeComponentValue::MangaChapterList {
		page_size: Some(5),
		entries: chapters,
		listing: None,
	};
	Ok(())
}

fn manga(element: &Element, params: &Params) -> Option<Manga> {
	let child;
	let link = if element.attr("href").is_some() {
		element
	} else {
		child = element.select_first("a[href*='/series/']")?;
		&child
	};
	let href = link.attr("href")?;
	let path = href
		.strip_prefix(params.base_url.as_ref())
		.unwrap_or(&href)
		.strip_prefix("/series/")?;
	let slug = path
		.split(['/', '?', '#'])
		.next()
		.filter(|s| !s.trim().is_empty())?;
	let title = link
		.attr("title")
		.filter(|s| !s.trim().is_empty())
		.or_else(|| {
			element
				.select_first("h2, h3, p.display-6")
				.and_then(|e| e.text())
		})
		.or_else(|| element.select_first("img").and_then(|e| e.attr("alt")))?;
	if title.trim().is_empty() {
		return None;
	}
	Some(Manga {
		key: format!("/series/{slug}"),
		title: title.trim().into(),
		cover: element.select_first("img").and_then(|img| {
			img.attr("data-fallback-src")
				.filter(|s| s.starts_with("https://") && !s.contains("no-cover"))
				.or_else(|| img.attr("abs:src"))
		}),
		..Default::default()
	})
}

pub fn parse(html: &Document, params: &Params) -> Result<HomeLayout> {
	let mut components = Vec::new();
	let mut slides: Vec<_> = html
		.select(&params.home_slider_selector)
		.map(|slides| {
			slides
				.filter_map(|e| {
					let mut manga = manga(&e, params)?;
					manga.description = e
						.select_first("h3 + p, .text-lg, .em-hero-copy p.d-none")
						.and_then(|p| p.text())
						.map(|s| s.trim().into())
						.filter(|s: &String| !s.is_empty());
					let index = e
						.attr("data-swiper-slide-index")
						.and_then(|s| s.parse::<usize>().ok())
						.unwrap_or(0);
					Some((index, manga))
				})
				.collect()
		})
		.unwrap_or_default();
	// Looping carousels can rotate and duplicate their DOM slides.
	if !slides.is_sorted_by_key(|(index, _)| *index) {
		slides.sort_by_key(|(index, _)| *index);
	}
	let mut entries: Vec<Manga> = Vec::new();
	for (_, manga) in slides {
		if !entries.iter().any(|entry| entry.key == manga.key) {
			entries.push(manga);
		}
	}
	if !entries.is_empty() {
		components.push(HomeComponent {
			value: HomeComponentValue::BigScroller {
				entries,
				auto_scroll_interval: None,
			},
			..Default::default()
		});
	}
	if let Some(headings) = html.select("main h1, main h2, main h3") {
		for heading in headings {
			let Some(title) = heading
				.text()
				.map(|s| s.trim().into())
				.filter(|s: &String| params.home_section_titles.contains(&s.as_str()))
			else {
				continue;
			};
			if components
				.iter()
				.any(|c| c.title.as_deref() == Some(&title))
			{
				continue;
			}
			let mut parent = heading.parent();
			let mut entries: Vec<Link> = Vec::new();
			for _ in 0..3 {
				let Some(block) = parent else {
					break;
				};
				if block.tag_name().as_deref() == Some("main") {
					break;
				}
				let neighbors = block.select("h1, h2, h3").is_some_and(|hs| {
					hs.filter_map(|h| h.text()).any(|t| {
						t.trim() != title && params.home_section_titles.contains(&t.trim())
					})
				});
				if neighbors {
					break;
				}
				if let Some(cards) = block.select("a[href*='/series/']") {
					for card in cards {
						if card.select_first("img").is_none() {
							continue;
						}
						if let Some(manga) = manga(&card, params)
							&& !entries.iter().any(|entry| matches!(&entry.value, Some(LinkValue::Manga(existing)) if existing.key == manga.key)) { entries.push(Link::from(manga)); }
					}
				}
				if !entries.is_empty() {
					break;
				}
				parent = block.parent();
			}
			if entries.is_empty() {
				continue;
			}
			let ranking = matches!(title.as_str(), "Most Popular" | "Tendencias");
			let paged = ranking
				|| matches!(
					title.as_str(),
					"Latest Releases" | "Latest Updates" | "Últimas Actualizaciones"
				);
			components.push(HomeComponent {
				title: Some(title),
				value: if paged {
					HomeComponentValue::MangaList {
						ranking,
						page_size: Some(if ranking { 3 } else { 5 }),
						entries,
						listing: None,
					}
				} else {
					HomeComponentValue::Scroller {
						entries,
						listing: None,
					}
				},
				..Default::default()
			});
		}
	}
	if components.is_empty() {
		bail!("Home sections not found");
	}
	Ok(HomeLayout { components })
}
