use super::*;
use aidoku::{
	FilterKind, HomeComponentValue, LinkValue, MangaStatus, Viewer,
	alloc::{String, vec},
};
use aidoku_test::aidoku_test;
use home::{Feed, HomeData, layout};

fn card(id: &str, extra: Value) -> MangaCard {
	let mut value = json!({ "_id": id, "name": id, "thumbnail": "mcovers/m_tbs/x/001.webp" });
	if let (Some(object), Value::Object(extra)) = (value.as_object_mut(), extra) {
		object.extend(extra);
	}
	serde_json::from_value(value).unwrap()
}

// 2026-11-14 12:00 UTC
const NOW: i64 = 1_794_657_600;

#[aidoku_test]
fn review_home_rows_follow_the_requested_order() {
	let rows = layout(HomeData {
		popular: Some(vec![card(
			"hero",
			json!({ "description": "Line one<br>\nLine two", "genres": ["Romance", "Action"], "status": "Releasing" }),
		)]),
		random: Some(vec![card("random", json!({}))]),
		feeds: vec![
			(Feed::Adaptations, vec![card("adaptation", json!({}))]),
			(Feed::Boosted, vec![card("boosted", json!({}))]),
			(Feed::Saved, vec![card("saved", json!({}))]),
			(
				Feed::Latest,
				vec![card(
					"latest",
					json!({
						"lastChapterInfo": { "sub": { "chapterString": "56" } },
						"lastChapterDate": { "sub": { "year": 2026, "month": 10, "date": 4, "hour": 16, "minute": 31, "second": 43 } },
					}),
				)],
			),
			(Feed::Trending, vec![card("trending", json!({}))]),
			(Feed::Year, vec![card("year", json!({}))]),
		],
		now: NOW,
	});
	let titles: Vec<_> = rows
		.components
		.iter()
		.map(|component| component.title.clone().unwrap_or_default())
		.collect();
	assert_eq!(
		titles,
		[
			"Popular Weekly",
			"Manga 2026",
			"Trending",
			"Random",
			"Latest Updates",
			"Most Saved",
			"Most Boosted",
			"Fall 2026 Anime",
		]
	);
	let HomeComponentValue::BigScroller { entries, .. } = &rows.components[0].value else {
		panic!("Expected the popular hero")
	};
	assert_eq!(
		entries[0].description.as_deref(),
		Some("Line one\n\nLine two")
	);
	assert_eq!(
		entries[0].tags,
		Some(vec![String::from("Action"), String::from("Romance")])
	);
	assert_eq!(entries[0].status, MangaStatus::Ongoing);
	let HomeComponentValue::MangaChapterList {
		entries, page_size, ..
	} = &rows.components[4].value
	else {
		panic!("Expected the latest chapters")
	};
	assert_eq!(*page_size, Some(5));
	assert_eq!(entries[0].chapter.key, "56");
	assert_eq!(entries[0].chapter.chapter_number, Some(56.));
	assert!(entries[0].chapter.date_uploaded.is_some());
	for index in [1, 2, 3, 5, 6, 7] {
		let HomeComponentValue::Scroller { entries, .. } = &rows.components[index].value else {
			panic!("Expected a card row at {index}")
		};
		let Some(LinkValue::Manga(manga)) = &entries[0].value else {
			panic!("Expected a series link")
		};
		assert!(entries[0].subtitle.is_none());
		assert!(manga.description.is_none() && manga.tags.is_none());
	}
}

fn detail(extra: Value) -> MangaDetail {
	let mut value = json!({ "name": "Series" });
	if let (Some(object), Value::Object(extra)) = (value.as_object_mut(), extra) {
		object.extend(extra);
	}
	serde_json::from_value(value).unwrap()
}

#[aidoku_test]
fn review_viewer_follows_type_country_and_strip_tags() {
	let viewer_of = |extra: Value| detail(extra).into_details("x").unwrap().viewer;
	assert_eq!(viewer_of(json!({ "type": "Manhwa" })), Viewer::Webtoon);
	assert_eq!(viewer_of(json!({ "type": "Manhua" })), Viewer::Webtoon);
	assert_eq!(viewer_of(json!({ "type": "Manga" })), Viewer::RightToLeft);
	assert_eq!(
		viewer_of(json!({ "countryOfOrigin": "KR" })),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_of(json!({ "countryOfOrigin": "JP" })),
		Viewer::RightToLeft
	);
	assert_eq!(
		viewer_of(json!({ "countryOfOrigin": "EN" })),
		Viewer::Unknown
	);
	assert_eq!(
		viewer_of(json!({ "type": "Manga", "genres": ["Webtoons"] })),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_of(json!({ "type": "Manga", "tags": ["format:long_strip"] })),
		Viewer::Webtoon
	);
}

#[aidoku_test]
fn review_detail_tags_are_readable_and_sorted() {
	let manga = detail(json!({
		"genres": ["Romance", "Action", "action"],
		"tags": ["format:long_strip", "theme:anachronism"],
	}))
	.into_details("x")
	.unwrap();
	assert_eq!(
		manga.tags,
		Some(vec![
			String::from("Action"),
			String::from("Anachronism"),
			String::from("Long Strip"),
			String::from("Romance"),
		])
	);
	assert_eq!(manga.url.as_deref(), Some("https://mkissa.to/manga/x"));
}

#[aidoku_test]
fn review_untitled_cards_are_dropped() {
	assert!(Manga::try_from(card("", json!({}))).is_err());
	assert!(Manga::try_from(card("x", json!({ "name": " " }))).is_err());
	assert!(detail(json!({ "name": "" })).into_details("x").is_err());
	let rows = layout(HomeData {
		popular: None,
		random: Some(vec![card("x", json!({ "name": "" }))]),
		feeds: Vec::new(),
		now: NOW,
	});
	assert!(rows.components.is_empty());
}

#[aidoku_test]
fn review_chapters_match_episode_notes() {
	let data: ChaptersData = serde_json::from_value(json!({
		"manga": { "availableChaptersDetail": { "sub": ["12.5", "12", "3"] } },
		"episodeInfos": [
			{ "episodeIdNum": 3, "notes": "Ep 3" },
			{ "episodeIdNum": 12, "notes": "[S2] ep. 12 - The Return" },
			{ "episodeIdNum": "12.5", "notes": "EP12.5 - Extra &amp; Notes" },
		],
	}))
	.unwrap();
	let chapters = parse_chapters(data, "x");
	let titles: Vec<_> = chapters
		.iter()
		.map(|chapter| chapter.title.as_deref())
		.collect();
	assert_eq!(titles, [Some("Extra & Notes"), Some("The Return"), None]);
	assert_eq!(chapters[1].chapter_number, Some(12.));
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://mkissa.to/manga/x/chapter-12.5-sub")
	);
}

#[aidoku_test]
fn review_missing_rows_drop_out() {
	let rows = layout(HomeData {
		popular: None,
		random: Some(Vec::new()),
		feeds: vec![(Feed::Trending, vec![card("trending", json!({}))])],
		now: NOW,
	});
	assert_eq!(rows.components.len(), 1);
	assert_eq!(rows.components[0].title.as_deref(), Some("Trending"));
}

#[aidoku_test]
fn review_season_titles_follow_calendar_quarters() {
	assert_eq!(helpers::season_and_year(NOW), ("Fall", 2026));
	// 2027-01-01 00:00 UTC
	assert_eq!(helpers::season_and_year(1_798_761_600), ("Winter", 2027));
	// 2026-06-30 23:59 UTC
	assert_eq!(helpers::season_and_year(1_782_863_940), ("Spring", 2026));
}

#[aidoku_test]
fn review_deep_links_keep_series_and_chapter_keys() {
	let source = Mkissa::new();
	assert_eq!(
		source
			.handle_deep_link("https://mkissa.to/manga/J5aN5NY2Zsrk6DLsX/chapter-12.5-sub".into())
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: "J5aN5NY2Zsrk6DLsX".into(),
			key: "12.5".into(),
		})
	);
	assert_eq!(
		source
			.handle_deep_link("https://mkissa.to/manga/J5aN5NY2Zsrk6DLsX".into())
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "J5aN5NY2Zsrk6DLsX".into()
		})
	);
}

#[aidoku_test]
fn home_loads_every_row() {
	let rows = Mkissa::new().get_home().unwrap();
	assert_eq!(
		rows.components.len(),
		8,
		"{:?}",
		rows.components.iter().map(|c| &c.title).collect::<Vec<_>>()
	);
}

#[aidoku_test]
fn listings_page_through_each_feed() {
	for id in [
		"latest",
		"manga_year",
		"trending",
		"most_saved",
		"most_boosted",
		"adaptations",
	] {
		let first = Mkissa::new()
			.get_manga_list(
				Listing {
					id: id.into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		assert!(!first.entries.is_empty(), "{id}");
		assert!(first.has_next_page, "{id}");
		let second = Mkissa::new()
			.get_manga_list(
				Listing {
					id: id.into(),
					..Default::default()
				},
				2,
			)
			.unwrap();
		assert_ne!(first.entries[0].key, second.entries[0].key, "{id}");
	}
}

#[aidoku_test]
fn search_finds_a_title_and_a_pasted_link() {
	let source = Mkissa::new();
	let results = source
		.get_search_manga_list(Some("Berserk".into()), 1, Vec::new())
		.unwrap();
	assert!(
		results
			.entries
			.iter()
			.any(|manga| manga.key == "J5aN5NY2Zsrk6DLsX")
	);
	let pasted = source
		.get_search_manga_list(
			Some("https://mkissa.to/manga/J5aN5NY2Zsrk6DLsX".into()),
			1,
			Vec::new(),
		)
		.unwrap();
	assert_eq!(pasted.entries.len(), 1);
	assert_eq!(pasted.entries[0].title, "Berserk");
	assert!(pasted.entries[0].description.is_none());
}

#[aidoku_test]
fn genre_filter_narrows_results() {
	let results = Mkissa::new()
		.get_search_manga_list(
			None,
			1,
			vec![FilterValue::MultiSelect {
				id: "genres".into(),
				included: vec!["Sports".into()],
				excluded: Vec::new(),
			}],
		)
		.unwrap();
	let details = Mkissa::new()
		.get_manga_update(results.entries[0].clone(), true, false)
		.unwrap();
	assert!(
		details
			.tags
			.unwrap_or_default()
			.iter()
			.any(|tag| tag == "Sports")
	);
}

fn search(filters: Vec<FilterValue>) -> MangaPageResult {
	Mkissa::new()
		.get_search_manga_list(None, 1, filters)
		.unwrap()
}

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
}

fn keys(result: &MangaPageResult) -> Vec<String> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.clone())
		.collect()
}

#[aidoku_test]
fn review_filters_follow_the_site_panel() {
	let filters: Value = serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let filter = |id: &str| {
		filters
			.as_array()
			.unwrap()
			.iter()
			.find(|filter| filter["id"] == id)
			.unwrap()
			.clone()
	};
	// Genres and tags only include on the site
	assert_eq!(filter("genres")["canExclude"], false);
	let genres = filter("genres")["options"].clone();
	assert_eq!(genres[45], "Yuri");
	assert_eq!(genres[46], "Suspense");
	assert_eq!(filter("chapters")["ids"][8], "2001");
	assert_eq!(
		filter("season")["options"],
		json!(["Any", "Winter", "Spring", "Summer", "Fall"])
	);
}

#[aidoku_test]
fn review_update_days_are_whole_days() {
	let (today, today_end) = day_range(0, NOW).unwrap();
	let (yesterday, yesterday_end) = day_range(1, NOW).unwrap();
	assert!(today <= NOW && NOW <= today_end);
	assert_eq!(today_end - today, 86399);
	assert_eq!(today - yesterday, 86400);
	assert_eq!(yesterday_end + 1, today);
}

#[aidoku_test]
fn site_filters_narrow_results() {
	let unfiltered = keys(&search(Vec::new()));
	let long = search(vec![select("chapters", "2001")]);
	assert!(!long.entries.is_empty() && !long.has_next_page);
	let short = search(vec![select("chapters", "1-10")]);
	assert_ne!(keys(&short), unfiltered);
	let season = search(vec![select("season", "Fall")]);
	assert_ne!(keys(&season), unfiltered);
	let dated = search(vec![
		select("season", "Spring"),
		FilterValue::Text {
			id: "year".into(),
			value: "2015".into(),
		},
	]);
	assert_ne!(keys(&dated), keys(&season));
	assert!(!search(vec![select("day", "1")]).entries.is_empty());
	assert!(
		Mkissa::new()
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Text {
					id: "year".into(),
					value: "soon".into(),
				}],
			)
			.is_err()
	);
}

#[aidoku_test]
fn trending_tags_filter_and_tag_taps_search_by_slug() {
	let filters = Mkissa::new().get_dynamic_filters().unwrap();
	let FilterKind::MultiSelect {
		ids: Some(ids),
		options,
		can_exclude,
		..
	} = &filters[0].kind
	else {
		panic!("Expected the tags multi-select")
	};
	assert!(!options.is_empty() && !*can_exclude);
	let tagged = search(vec![FilterValue::MultiSelect {
		id: "tags".into(),
		included: vec![ids[0].clone().into_owned()],
		excluded: Vec::new(),
	}]);
	assert!(!tagged.entries.is_empty());
	let tapped = search(vec![select("genre", "Long Strip")]);
	assert!(!tapped.entries.is_empty());
	let none = search(vec![FilterValue::MultiSelect {
		id: "tags".into(),
		included: vec!["theme:zzzz_none".into()],
		excluded: Vec::new(),
	}]);
	assert!(none.entries.is_empty());
}

#[aidoku_test]
fn details_and_chapters_load() {
	let manga = Mkissa::new()
		.get_manga_update(
			Manga {
				key: "J5aN5NY2Zsrk6DLsX".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "Berserk");
	assert!(manga.description.is_some());
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.viewer, Viewer::RightToLeft);
	assert!(
		manga
			.tags
			.as_ref()
			.is_some_and(|tags| tags.iter().all(|tag| !tag.contains(':')))
	);
	let manhwa = Mkissa::new()
		.get_manga_update(
			Manga {
				key: "MrGT6Smvdm4Dp7t4J".into(),
				..Default::default()
			},
			true,
			false,
		)
		.unwrap();
	assert_eq!(manhwa.viewer, Viewer::Webtoon);
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() > 300);
	assert!(chapters[0].chapter_number > chapters[1].chapter_number);
}

#[aidoku_test]
fn an_unknown_series_fails() {
	assert!(
		Mkissa::new()
			.get_manga_update(
				Manga {
					key: "doesnotexist000".into(),
					..Default::default()
				},
				true,
				false,
			)
			.is_err()
	);
}
