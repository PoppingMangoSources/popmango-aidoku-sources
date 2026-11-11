#![no_std]

#[cfg(test)]
mod contracts {
	use aidoku::{
		AidokuError, Chapter, HomeComponentValue, Manga, PageContent,
		alloc::{String, Vec, format},
		imports::html::Html,
	};
	use aidoku_test::aidoku_test;
	use keyoapp::{Impl, ImplV2, Params, home};

	struct V1;
	impl Impl for V1 {
		fn new() -> Self {
			Self
		}
		fn params(&self) -> Params {
			Params::default()
		}
	}
	struct V2;
	impl ImplV2 for V2 {
		fn new() -> Self {
			Self
		}
		fn params(&self) -> Params {
			Params::default()
		}
	}
	fn params(path: &str) -> Params {
		Params {
			base_url: format!("{}{path}", env!("AIDOKU_REVIEW_BASE_URL")).into(),
			..Default::default()
		}
	}
	fn manga(kind: &str) -> Manga {
		Manga {
			key: format!("/{kind}"),
			..Default::default()
		}
	}
	fn names(values: &[&str]) -> Option<Vec<String>> {
		Some(values.iter().map(|value| (*value).into()).collect())
	}
	fn assert_credits(manga: &Manga) {
		assert_eq!(manga.authors, names(&["First", "Second"]));
		assert_eq!(manga.artists, names(&["Third", "First"]));
	}

	#[aidoku_test]
	fn review_contract_short_and_expanded_synopses() {
		for (kind, expected) in [
			("short", Some("Full synopsis")),
			("expanded", Some("Full synopsis")),
			("empty", None),
			("missing", None),
		] {
			let manga = V1
				.get_manga_update(&params("/keyoapp"), manga(kind), true, false)
				.unwrap();
			assert_eq!(manga.description.as_deref(), expected);
		}
	}

	#[aidoku_test]
	fn review_contract_credits_split_trim_deduplicate_and_keep_roles() {
		assert_credits(
			&V1.get_manga_update(&params("/keyoapp"), manga("short"), true, false)
				.unwrap(),
		);
		for kind in ["by", "blocks", "empty-by"] {
			assert_credits(
				&V2.get_manga_update(&params("/keyoapp-v2"), manga(kind), true, false)
					.unwrap(),
			);
		}
	}

	#[aidoku_test]
	fn review_contract_blank_detail_identity_fails() {
		let result = V1.get_manga_update(&params("/keyoapp"), manga("blank-title"), true, false);
		assert!(
			matches!(result, Err(AidokuError::Message(message)) if message == "Series not found")
		);
		for kind in ["blank-title", "blank-slug"] {
			let result = V2.get_manga_update(&params("/keyoapp-v2"), manga(kind), true, false);
			assert!(
				matches!(result, Err(AidokuError::Message(message)) if message == "Series not found")
			);
		}
	}

	#[aidoku_test]
	fn review_contract_newly_added_cannot_borrow_another_row() {
		let card = r#"<a class="group" href="/new" title="New"><img data-src="https://example.com/cover.jpg"></a>"#;
		for markup in [
			format!(
				r#"<section id="home-newly-added"><h2>Newly Added Series</h2>{card}</section>"#
			),
			format!(
				r#"<section id="home-newly-added"><h2>Newly Added Series</h2></section><div id="series-grid">{card}</div>"#
			),
		] {
			let layout = home::parse_home_v2(&params(""), &Html::parse(markup).unwrap());
			assert_eq!(layout.components.len(), 1);
			let HomeComponentValue::Scroller { entries, .. } = &layout.components[0].value else {
				panic!("Expected compact cards")
			};
			assert_eq!(entries.len(), 1);
			assert!(entries[0].subtitle.is_none());
		}
		let markup = format!(
			r#"<section id="home-newly-added"><h2>Newly Added Series</h2></section><section><h2>Other</h2></section><div id="series-grid">{card}</div>"#
		);
		assert!(
			home::parse_home_v2(&params(""), &Html::parse(markup).unwrap())
				.components
				.is_empty()
		);
	}

	#[aidoku_test]
	fn review_contract_section_walk_stays_within_three_ancestors() {
		for depth in [2, 3] {
			let markup = format!(
				r#"<div>{}<h2>Series</h2>{}<button title="Title"><a href="/series/title/" title="Title"></a></button></div>"#,
				"<div>".repeat(depth),
				"</div>".repeat(depth)
			);
			let layout = home::parse_home(&params(""), &Html::parse(markup).unwrap());
			assert_eq!(layout.components.len(), usize::from(depth == 2));
		}
	}

	#[aidoku_test]
	fn review_contract_home_periods_are_independent() {
		let layout = V2.get_home(&params("/keyoapp-home")).unwrap();
		for (title, expected) in [
			("Trending Now", "Today"),
			("Most Reviewed Series", "Monthly"),
		] {
			let component = layout
				.components
				.iter()
				.find(|component| component.title.as_deref() == Some(title))
				.unwrap();
			let HomeComponentValue::Scroller { entries, .. } = &component.value else {
				panic!("Expected cards")
			};
			assert_eq!(entries[0].title, expected);
			assert!(entries[0].subtitle.is_none());
		}
	}

	#[aidoku_test]
	fn review_contract_key_only_novels_preserve_text_and_images() {
		let chapter = || Chapter {
			key: "/novel".into(),
			..Default::default()
		};
		let pages = V1
			.get_page_list(&params("/keyoapp"), Manga::default(), chapter())
			.unwrap();
		assert_eq!(pages.len(), 1);
		assert!(
			matches!(&pages[0].content, PageContent::Text(text) if text == "**First**\n\n**Second**\n\n*Third*")
		);
		let pages = V2
			.get_page_list(&params("/keyoapp-v2"), Manga::default(), chapter())
			.unwrap();
		assert_eq!(pages.len(), 2);
		assert!(
			matches!(&pages[0].content, PageContent::Text(text) if text == "**First**\n\n**Second**\n\n*Third*")
		);
		assert!(
			matches!(&pages[1].content, PageContent::Url(url, _) if url == "https://example.com/page.jpg")
		);
	}
}
