#![expect(clippy::unwrap_used, clippy::panic)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, Home, HomeComponentValue, Listing, ListingProvider,
	Manga, MangaStatus, alloc::vec,
};
use aidoku_test::aidoku_test;

const MANGA_KEY: &str = "/one-piece";

fn source() -> mangak::MangaK<MangaK> {
	Source::new()
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn keys(result: &MangaPageResult) -> Vec<&str> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.as_str())
		.collect()
}

fn multi(id: &str, included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: excluded.iter().map(|value| (*value).into()).collect(),
	}
}

// One Piece is an ongoing shounen manga with over a thousand chapters by Oda Eiichiro
fn finds_one_piece(filter: FilterValue) -> bool {
	keys(&search(Some("one piece"), vec![filter])).contains(&MANGA_KEY)
}

#[aidoku_test]
fn browse_pages() {
	let source = source();
	let first = source.get_search_manga_list(None, 1, vec![]).unwrap();
	let second = source.get_search_manga_list(None, 2, vec![]).unwrap();
	// the template sends no limit, so pages are the API's default size
	assert_eq!(first.entries.len(), 20);
	assert!(first.has_next_page && second.has_next_page);
	assert!(keys(&first).iter().all(|key| !keys(&second).contains(key)));
	let manga = &first.entries[0];
	assert!(manga.key.starts_with('/') && !manga.title.is_empty() && manga.cover.is_some());
}

#[aidoku_test]
fn query() {
	assert_eq!(keys(&search(Some("one piece"), vec![]))[0], MANGA_KEY);
}

#[aidoku_test]
fn every_sort() {
	for index in 0..10 {
		let result = search(
			None,
			vec![FilterValue::Sort {
				id: "sort".into(),
				index,
				ascending: false,
			}],
		);
		assert!(!result.entries.is_empty(), "sort {index} returned nothing");
	}
	let chapters = search(
		None,
		vec![FilterValue::Sort {
			id: "sort".into(),
			index: 9,
			ascending: false,
		}],
	);
	let latest = search(None, vec![]);
	assert_ne!(keys(&chapters), keys(&latest));
}

#[aidoku_test]
fn genres() {
	assert!(finds_one_piece(multi(
		"genres",
		&["action", "adventure"],
		&[]
	)));
	assert!(!finds_one_piece(multi("genres", &["yuri"], &[])));
	assert!(!finds_one_piece(multi("genres", &[], &["shounen"])));
	assert!(finds_one_piece(multi("genres", &[], &["yuri"])));
}

#[aidoku_test]
fn types() {
	assert!(finds_one_piece(multi("type", &["manga"], &[])));
	assert!(finds_one_piece(multi("type", &["manhwa", "manga"], &[])));
	assert!(!finds_one_piece(multi("type", &["manhwa", "manhua"], &[])));
}

#[aidoku_test]
fn content_ratings() {
	// the site rates One Piece suggestive
	assert!(finds_one_piece(multi(
		"content_rating",
		&["suggestive"],
		&[]
	)));
	assert!(finds_one_piece(multi(
		"content_rating",
		&["safe", "suggestive"],
		&[]
	)));
	for rating in ["safe", "erotica", "pornographic"] {
		assert!(!finds_one_piece(multi("content_rating", &[rating], &[])));
	}
}

#[aidoku_test]
fn status() {
	let select = |value: &str| FilterValue::Select {
		id: "status".into(),
		value: value.into(),
	};
	assert!(finds_one_piece(select("ongoing")));
	assert!(!finds_one_piece(select("completed")));
	assert!(finds_one_piece(select("")));
}

#[aidoku_test]
fn demographics() {
	assert!(finds_one_piece(multi("demographic", &["shounen"], &[])));
	assert!(finds_one_piece(multi(
		"demographic",
		&["shoujo", "shounen"],
		&[]
	)));
	assert!(!finds_one_piece(multi(
		"demographic",
		&["shoujo", "josei"],
		&[]
	)));
}

#[aidoku_test]
fn formats() {
	assert!(!finds_one_piece(multi("format", &["oneshot"], &[])));
	let oneshots = search(None, vec![multi("format", &["oneshot"], &[])]);
	assert!(!oneshots.entries.is_empty());
}

#[aidoku_test]
fn chapter_range() {
	let range = |from: Option<f32>, to: Option<f32>| FilterValue::Range {
		id: "chapters".into(),
		from,
		to,
	};
	assert!(finds_one_piece(range(Some(1000.0), None)));
	assert!(!finds_one_piece(range(None, Some(10.0))));
	assert!(finds_one_piece(range(Some(1000.0), Some(5000.0))));
}

#[aidoku_test]
fn author() {
	let author = |value: &str| FilterValue::Text {
		id: "author".into(),
		value: value.into(),
	};
	assert!(finds_one_piece(author("Oda Eiichiro")));
	assert!(!finds_one_piece(author("Togashi")));
	assert!(finds_one_piece(author("")));
}

#[aidoku_test]
fn listings() {
	let source = source();
	let listing = |id: &str| Listing {
		id: id.into(),
		..Default::default()
	};
	let trending = source.get_manga_list(listing("trending"), 1).unwrap();
	assert!(!trending.entries.is_empty() && !trending.has_next_page);
	for id in ["popular_updates", "new", "latest"] {
		let first = source.get_manga_list(listing(id), 1).unwrap();
		let second = source.get_manga_list(listing(id), 2).unwrap();
		assert!(first.has_next_page, "listing {id} has one page");
		assert!(keys(&first).iter().all(|key| !keys(&second).contains(key)));
	}
	assert_ne!(
		keys(&source.get_manga_list(listing("new"), 1).unwrap()),
		keys(&source.get_manga_list(listing("latest"), 1).unwrap())
	);
	assert!(source.get_manga_list(listing("today"), 1).is_err());
}

#[aidoku_test]
fn review_featured_genres() {
	let genres = Filter {
		id: "genres".into(),
		options: vec!["Boys Love".into(), "Drama".into(), "Slice of life".into()],
		ids: vec!["boys-love".into(), "drama".into(), "slice-of-life".into()],
	};
	let tags = |slugs: Option<&str>| {
		Item {
			genre_slugs: slugs.map(Into::into),
			..Default::default()
		}
		.into_featured(&genres)
		.tags
	};
	// Names keep the feed's order, which is the order details show
	assert_eq!(
		tags(Some("slice-of-life,boys-love,drama")),
		Some(vec![
			"Slice of life".into(),
			"Boys Love".into(),
			"Drama".into()
		])
	);
	assert_eq!(tags(None), None);
}

#[aidoku_test]
fn home() {
	let home = source().get_home().unwrap();
	let titles = home
		.components
		.iter()
		.map(|component| component.title.as_deref().unwrap())
		.collect::<Vec<_>>();
	assert_eq!(
		titles,
		[
			"Hot this week",
			"Trending Today",
			"Popular Updates",
			"New Manga",
			"Recently Updated"
		]
	);
	let HomeComponentValue::BigScroller { entries, .. } = &home.components[0].value else {
		panic!("hot this week is not a big scroller");
	};
	assert_eq!(entries.len(), 5);
	assert!(entries.iter().all(|manga| manga.description.is_some()));
	// Opening a card must leave its tags as they are
	for featured in entries {
		let details = source()
			.get_manga_update(
				Manga {
					key: featured.key.clone(),
					..Default::default()
				},
				true,
				false,
			)
			.unwrap();
		assert!(featured.tags.is_some());
		assert_eq!(featured.tags, details.tags, "{}", featured.key);
	}
	for index in [1, 3] {
		let HomeComponentValue::Scroller { entries, listing } = &home.components[index].value
		else {
			panic!("section {index} is not a scroller");
		};
		assert!(!entries.is_empty() && listing.is_some());
		assert!(entries.iter().all(|link| link.subtitle.is_none()));
	}
	for index in [2, 4] {
		let HomeComponentValue::MangaChapterList { entries, .. } = &home.components[index].value
		else {
			panic!("section {index} is not a chapter list");
		};
		assert!(!entries.is_empty());
		for entry in entries {
			assert!(entry.chapter.key.starts_with(&entry.manga.key));
			assert!(entry.chapter.chapter_number.is_some() || entry.chapter.title.is_some());
		}
	}
}

#[aidoku_test]
fn details_and_pages() {
	let source = source();
	let manga = source
		.get_manga_update(
			Manga {
				key: MANGA_KEY.into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "One Piece");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert!(
		manga
			.authors
			.unwrap()
			.iter()
			.any(|name| name == "Oda Eiichiro")
	);
	assert!(manga.tags.unwrap().iter().any(|tag| tag == "Adventure"));
	assert_eq!(manga.url.as_deref(), Some("https://mangak.io/one-piece"));
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() > 1000);
	assert!(chapters[0].chapter_number.unwrap() > 1000.0);

	// a fresh chapter carrying only its key
	let pages = source
		.get_page_list(
			Manga {
				key: MANGA_KEY.into(),
				..Default::default()
			},
			Chapter {
				key: chapters[0].key.clone(),
				..Default::default()
			},
		)
		.unwrap();
	assert!(pages.len() > 5);
}

#[aidoku_test]
fn chapters_only() {
	let manga = source()
		.get_manga_update(
			Manga {
				key: MANGA_KEY.into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert!(manga.title.is_empty());
	assert!(manga.chapters.unwrap().len() > 1000);
}

#[aidoku_test]
fn unknown_key() {
	assert!(
		source()
			.get_manga_update(
				Manga {
					key: "/no-such-series-zzz".into(),
					..Default::default()
				},
				true,
				true,
			)
			.is_err()
	);
}

#[aidoku_test]
fn deep_links() {
	let source = source();
	assert!(matches!(
		source.handle_deep_link("https://mangak.io/one-piece".into()).unwrap(),
		Some(DeepLinkResult::Manga { key }) if key == MANGA_KEY
	));
	assert!(matches!(
		source
			.handle_deep_link("https://mangak.io/one-piece/chapter-1197".into())
			.unwrap(),
		Some(DeepLinkResult::Chapter { manga_key, key })
			if manga_key == MANGA_KEY && key == "/one-piece/chapter-1197"
	));
}
