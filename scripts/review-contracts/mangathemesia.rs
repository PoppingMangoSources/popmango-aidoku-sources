#![no_std]

#[cfg(test)]
mod contracts {
	use aidoku::{
		Chapter, DeepLinkResult, HomeComponentValue, LinkValue, Manga, PageContent, Viewer,
		alloc::format, imports::html::Html,
	};
	use aidoku_test::aidoku_test;
	use mangathemesia::{Impl, Params, child_theme};

	struct Site;
	impl Impl for Site {
		fn new() -> Self {
			Self
		}
		fn params(&self) -> Params {
			Params::default()
		}
	}
	fn params() -> Params {
		Params {
			base_url: env!("AIDOKU_REVIEW_BASE_URL").into(),
			manga_url_directory: "/series".into(),
			..Default::default()
		}
	}

	#[aidoku_test]
	fn review_contract_viewer_override_preserves_other_consumers() {
		for (kind, expected) in [
			("Manga", Viewer::RightToLeft),
			("Manhwa", Viewer::Webtoon),
			("Comic", Viewer::LeftToRight),
		] {
			for viewer_override in [None, Some(Viewer::Webtoon)] {
				let params = Params {
					viewer_override,
					series_type_selector: ".type".into(),
					..params()
				};
				let manga = Site
					.get_manga_update(
						&params,
						Manga {
							key: format!("/mangathemesia/{kind}"),
							..Default::default()
						},
						true,
						false,
					)
					.unwrap();
				assert_eq!(manga.viewer, viewer_override.unwrap_or(expected));
			}
		}
	}

	#[aidoku_test]
	fn review_contract_key_only_novels_preserve_paragraphs_and_adjacent_emphasis() {
		for (kind, expected) in [
			("novel", "**First**\n\n**Second**\n\n*Third*"),
			("novel-adjacent", "***FirstSecond***"),
		] {
			let pages = Site
				.get_page_list(
					&params(),
					Manga::default(),
					Chapter {
						key: format!("/mangathemesia/{kind}"),
						..Default::default()
					},
				)
				.unwrap();
			assert_eq!(pages.len(), 1);
			assert!(matches!(&pages[0].content, PageContent::Text(text) if text == expected));
		}
	}

	#[aidoku_test]
	fn review_contract_child_home_has_compact_banners_and_five_row_chapters() {
		let html = Html::parse(r#"
		<div class="slidernew"><div class="swiper-slide"><a href="/series/banner/"><img alt="Banner" src="https://example.com/banner.jpg"></a><a href="/series/blank/"><img alt=" " src="https://example.com/blank.jpg"></a></div></div>
		<div class="bixbox"><div class="releases"><h2>Latest Comics</h2></div><div id="manga-posts">
		<div class="bsx"><a href="/series/new/" title="New"><img src="https://example.com/new.jpg"></a><div class="chapter-list"><a href="/locked/"><span class="epxs">Chapter 2</span><i class="fa-coins"></i></a><a href="/free/"><span class="epxs">Chapter 1</span></a></div></div>
		<div class="bsx"><a href=" " title="Blank"><img src="https://example.com/blank.jpg"></a><div class="chapter-list"><a href="/unrelated/"><span class="epxs">Chapter 1</span></a></div></div>
		</div></div>"#).unwrap();
		let params = Params {
			banner_width: Some(400),
			banner_height: Some(200),
			..params()
		};
		let layout = child_theme::parse_home(&params, &html);
		assert_eq!(layout.components.len(), 2);
		let HomeComponentValue::ImageScroller {
			links,
			width,
			height,
			..
		} = &layout.components[0].value
		else {
			panic!("Expected banners")
		};
		assert_eq!((*width, *height), (Some(400), Some(200)));
		assert_eq!(links.len(), 1);
		assert!(links[0].subtitle.is_none());
		assert!(
			matches!(&links[0].value, Some(LinkValue::Manga(manga)) if manga.key == "/series/banner/")
		);
		let HomeComponentValue::MangaChapterList {
			entries, page_size, ..
		} = &layout.components[1].value
		else {
			panic!("Expected chapters")
		};
		assert_eq!(*page_size, Some(5));
		assert_eq!(entries.len(), 1);
		assert_eq!(entries[0].chapter.key, "/free/");
		assert!(entries[0].manga.authors.is_none() && entries[0].manga.description.is_none());
	}

	#[aidoku_test]
	fn review_contract_chapter_links_keep_canonical_keys() {
		let params = params();
		let html = Html::parse(format!(r#"<link rel="canonical" href="{}/chapter-one/"><div id="readerarea"></div><div class="ts-breadcrumb"><a href="/series/">Catalogue</a><a href="/series/title/?share=1">Title</a></div>"#, params.base_url)).unwrap();
		assert_eq!(
			child_theme::parse_deep_link(
				&params,
				&html,
				&format!("{}/chapter-one/?share=1#reader", params.base_url)
			),
			Some(DeepLinkResult::Chapter {
				manga_key: "/series/title/".into(),
				key: "/chapter-one/".into()
			})
		);
	}
}
