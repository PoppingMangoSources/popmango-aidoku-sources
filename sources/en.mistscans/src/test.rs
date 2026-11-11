#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, Manga,
	PageContent, alloc::vec, imports::html::Html,
};
use aidoku_test::aidoku_test;

fn source() -> KeyoappV2<MistScans> {
	KeyoappV2::new()
}

#[aidoku_test]
fn review_v2_filters_match_the_site() {
	let filters = include_str!("../res/filters.json");
	assert!(filters.find("\"sort\"").unwrap() < filters.find("\"type\": \"select\"").unwrap());
	assert_eq!(filters.matches("\"canExclude\": false").count(), 1);
	assert!(filters.contains("\"canAscend\": false"));
	assert!(!filters.contains("\"comic\""));
	assert_eq!(BASE_URL, "https://mistscans.org");
	assert!(!include_str!("../res/source.json").contains("breakingChangeVersion"));
}

#[aidoku_test]
fn review_v2_home_keeps_newly_added_and_flat_keys() {
	let html = Html::parse(r#"<section><h2>Featured Series</h2><a href="https://mistscans.org/roxana" data-slide-card><h3>Roxana</h3><img data-src="https://example.com/cover.avif"></a></section><section id="home-newly-added"><h2>Newly Added Series</h2></section><div id="series-grid"><a href="https://mistscans.org/first-love-trap" class="group"><h2>First Love Trap</h2><img data-src="https://example.com/new.avif"></a></div>"#).unwrap();
	let home = keyoapp::home::parse_home_v2(&MistScans.params(), &html);
	assert_eq!(home.components.len(), 2);
	for (component, title, key) in [
		(&home.components[0], "Featured Series", "/roxana"),
		(
			&home.components[1],
			"Newly Added Series",
			"/first-love-trap",
		),
	] {
		assert_eq!(component.title.as_deref(), Some(title));
		let HomeComponentValue::Scroller { entries, .. } = &component.value else {
			panic!("Expected scroller");
		};
		assert_eq!(entries.len(), 1);
		assert!(entries[0].subtitle.is_none());
		assert!(
			matches!(&entries[0].value, Some(aidoku::LinkValue::Manga(m)) if m.key == key && m.description.is_none() && m.tags.is_none())
		);
	}
}

#[aidoku_test]
fn catalogue_search_filters_and_page_two() {
	let site = source();
	let first = site.get_search_manga_list(None, 1, vec![]).unwrap();
	let second = site.get_search_manga_list(None, 2, vec![]).unwrap();
	assert!(first.has_next_page && !second.entries.is_empty());
	assert!(
		first
			.entries
			.iter()
			.all(|m| !m.title.is_empty() && m.cover.is_some() && m.description.is_none())
	);
	assert!(
		second
			.entries
			.iter()
			.all(|m| !first.entries.iter().any(|first| first.key == m.key))
	);
	assert!(
		site.get_search_manga_list(Some("zzqxwv".into()), 1, vec![])
			.unwrap()
			.entries
			.is_empty()
	);
	let found = site
		.get_search_manga_list(Some("Roxana".into()), 1, vec![])
		.unwrap();
	assert!(found.entries.iter().any(|m| m.key == "/roxana"));
	for (id, value) in [
		("type", "novel"),
		("status", "completed"),
		("genre", "Romance"),
	] {
		let result = site
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Select {
					id: id.into(),
					value: value.into(),
				}],
			)
			.unwrap();
		assert!(!result.entries.is_empty());
		assert_ne!(result.entries, first.entries);
	}
}

#[aidoku_test]
fn details_chapters_key_only_pages_and_deep_links() {
	let site = source();
	let manga = site
		.get_manga_update(
			Manga {
				key: "/roxana".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "Roxana");
	assert!(manga.cover.is_some());
	assert_ne!(manga.status, aidoku::MangaStatus::Unknown);
	let chapters = manga.chapters.unwrap();
	assert!(!chapters.is_empty());
	let chapter_only = site
		.get_manga_update(
			Manga {
				key: "/roxana".into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert_eq!(
		chapter_only.chapters.as_ref().unwrap().len(),
		chapters.len()
	);
	for chapter in [chapters.first().unwrap(), chapters.last().unwrap()] {
		let pages = site
			.get_page_list(
				Manga {
					key: "/roxana".into(),
					..Default::default()
				},
				Chapter {
					key: chapter.key.clone(),
					..Default::default()
				},
			)
			.unwrap();
		assert!(!pages.is_empty());
		assert!(pages.iter().all(
			|p| matches!(&p.content, PageContent::Url(url, _) if url.starts_with("https://"))
		));
		assert_eq!(
			site.handle_deep_link(aidoku::alloc::format!("{BASE_URL}{}", chapter.key))
				.unwrap(),
			Some(DeepLinkResult::Chapter {
				manga_key: "/roxana".into(),
				key: chapter.key.clone()
			})
		);
	}
	assert_eq!(
		site.handle_deep_link("https://mistscans.org/roxana".into())
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "/roxana".into()
		})
	);
	let home = site.get_home().unwrap();
	assert!(
		home.components
			.iter()
			.any(|c| c.title.as_deref() == Some("Newly Added Series"))
	);
}
