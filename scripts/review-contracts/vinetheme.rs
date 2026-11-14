#![no_std]

#[cfg(test)]
mod contracts {
	use aidoku::{Chapter, ContentRating, FilterKind, FilterValue, HomeComponentValue, Listing, Manga, PageContent, alloc::{String, Vec, format, vec}, imports::defaults::{DefaultValue, defaults_set}};
	use aidoku_test::aidoku_test;
	use vinetheme::{Impl, Params};

	struct Site;

	impl Impl for Site {
		fn new() -> Self { Self }

		fn params(&self) -> Params { params() }
	}

	fn params() -> Params { Params { base_url: format!("{}/vine", env!("AIDOKU_REVIEW_BASE_URL")).into() } }

	fn manga(key: &str) -> Manga { Manga { key: key.into(), ..Default::default() } }

	#[aidoku_test]
	fn review_contract_reader_selects_pages_and_rejects_truncation() {
		let pages = Site.get_page_list(&params(), manga("title"), Chapter { key: "2".into(), ..Default::default() }).unwrap();
		assert_eq!(pages.len(), 2);
		assert!(matches!(&pages[1].content, PageContent::Url(url, _) if url.ends_with("/2.webp")));
		for key in ["missing", "empty", "locked", "fragments", "active-protection", "redacted"] {
			assert!(Site.get_page_list(&params(), manga("title"), Chapter { key: key.into(), ..Default::default() }).is_err());
		}
		let inactive = Site.get_page_list(&params(), manga("title"), Chapter { key: "inactive-protection".into(), ..Default::default() }).unwrap();
		assert_eq!(inactive, pages);
	}

	#[aidoku_test]
	fn review_contract_novel_text_references_and_paragraphs() {
		let pages = Site.get_page_list(&params(), manga("novel/title"), Chapter { key: "2".into(), ..Default::default() }).unwrap();
		assert_eq!(pages.len(), 1);
		assert!(matches!(&pages[0].content, PageContent::Text(text) if text == "**First**\n\n***Second***\n\nThird 界."));
		assert!(Site.get_page_list(&params(), manga("novel/title"), Chapter { key: "empty-html".into(), ..Default::default() }).is_err());
	}

	#[aidoku_test]
	fn review_contract_complete_updates_preserve_legacy_keys() {
		defaults_set("showLocked", DefaultValue::Bool(false));
		let details = Site.get_manga_update(&params(), manga("comic/title"), true, false).unwrap();
		assert_eq!(details.key, "comic/title");
		assert_eq!(details.authors, Some(vec![String::from("First"), String::from("Second")]));
		let empty = Site.get_manga_update(&params(), manga("empty-description"), true, false).unwrap();
		assert_eq!(empty.title, "Title");
		assert!(empty.description.is_none());
		let combined = Site.get_manga_update(&params(), manga("title"), true, true).unwrap();
		let chapters = combined.chapters.unwrap();
		assert_eq!(chapters.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), ["2", "1"]);
		let only = Site.get_manga_update(&params(), manga("title"), false, true).unwrap();
		assert!(only.title.is_empty());
		assert_eq!(only.chapters.unwrap(), chapters);
		defaults_set("showLocked", DefaultValue::Bool(true));
		let shown = Site.get_manga_update(&params(), manga("title"), false, true).unwrap().chapters.unwrap();
		assert_eq!(shown.len(), 3);
		assert!(shown[0].locked);
		defaults_set("showLocked", DefaultValue::Bool(false));
		for key in ["incomplete", "duplicate"] {
			assert!(Site.get_manga_update(&params(), manga(key), false, true).is_err());
		}
	}

	#[aidoku_test]
	fn review_contract_search_filters_and_listing_pagination() {
		let filters = vec![
			FilterValue::Sort { id: "sort".into(), index: 6, ascending: true },
			FilterValue::MultiSelect { id: "genre".into(), included: vec!["action".into()], excluded: vec!["romance".into()] },
			FilterValue::Text { id: "ch_min".into(), value: "20".into() },
		];
		let page = Site.get_search_manga_list(&params(), Some("word & café".into()), 1, filters).unwrap();
		assert_eq!(page.entries.len(), 1);
		let entry = &page.entries[0];
		assert_eq!(entry.content_rating, ContentRating::NSFW);
		assert!(entry.description.is_none() && entry.authors.is_none() && entry.tags.is_none());
		assert!(entry.cover.as_deref().unwrap().ends_with("/cover.webp"));
		let page_two = Site.get_manga_list(&params(), Listing { id: "sort=newest".into(), name: "New Series".into(), ..Default::default() }, 2).unwrap();
		assert!(!page_two.has_next_page);
		assert_eq!(page_two.entries[0].key, "title-2");
		let genres = Site.get_dynamic_filters(&params()).unwrap();
		assert!(matches!(&genres[0].kind, FilterKind::MultiSelect { can_exclude: true, is_genre: true, .. }));
		let tapped = Site.get_search_manga_list(&params(), None, 1, vec![FilterValue::Select { id: "genre".into(), value: "Action".into() }]).unwrap();
		assert_eq!(tapped.entries.len(), 1);
		let unknown = Site.get_search_manga_list(&params(), None, 1, vec![FilterValue::Select { id: "genre".into(), value: "Unknown".into() }]).unwrap();
		assert!(unknown.entries.is_empty());
		let empty = Site.get_search_manga_list(&params(), None, 3, vec![]).unwrap();
		assert_eq!(empty.entries.len(), 1);
		let pasted = format!("{}/series/comic/title?ref=home", params().base_url);
		let found = Site.get_search_manga_list(&params(), Some(pasted), 1, vec![]).unwrap();
		assert_eq!(found.entries[0].key, "title");
		assert!(found.entries[0].description.is_none() && found.entries[0].authors.is_none());
		let missing = format!("{}/series/comic/missing-title?ref=home", params().base_url);
		let fallback = Site.get_search_manga_list(&params(), Some(missing), 1, vec![]).unwrap();
		assert_eq!(fallback.entries[0].key, "fallback-title");
	}

	#[aidoku_test]
	fn review_contract_home_order_unique_cards_and_real_chapters() {
		defaults_set("showLocked", DefaultValue::Bool(false));
		let home = Site.get_home(&params()).unwrap();
		assert_eq!(home.components.iter().map(|c| c.title.as_deref().unwrap_or("Hero")).collect::<Vec<_>>(), ["Hero", "New Series", "Latest Novel Updates", "Latest Comic Updates", "Popular Today", "Top 10 in All"]);
		let HomeComponentValue::BigScroller { entries, .. } = &home.components[0].value else { panic!() };
		assert_eq!(entries.len(), 1);
		assert_eq!(entries[0].description.as_deref(), Some("Plain description."));
		for index in [2, 3] {
			let HomeComponentValue::MangaChapterList { entries, page_size, .. } = &home.components[index].value else { panic!() };
			assert_eq!(entries.len(), 1);
			assert_eq!(entries[0].chapter.key, "2");
			assert_eq!(entries[0].chapter.chapter_number, Some(2.0));
			assert!(entries[0].chapter.date_uploaded.is_some());
			assert!(entries[0].manga.tags.is_none() && entries[0].manga.description.is_none());
			assert!(page_size.unwrap() <= 5);
		}
		assert_eq!(home.components[2].title.as_deref(), Some("Latest Novel Updates"));
		assert_eq!(ContentRating::NSFW, entries[0].content_rating);
	}
}
