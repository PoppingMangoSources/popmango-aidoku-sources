#![no_std]
use aidoku::{
	Chapter, Manga, Page, PageContent, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	helpers::element::ElementHelpers,
	imports::{html::Html, net::Request, std::send_partial_result},
	prelude::*,
};
use madara::{Impl, Madara, Params, helpers::ElementImageAttr};

mod chapters;

use chapters::LOCK_SUFFIX;

const BASE_URL: &str = "https://rinkocomics.com";

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
}

register_source!(
	Madara<RinkoComics>,
	Home,
	DeepLinkHandler,
	ImageRequestProvider
);
