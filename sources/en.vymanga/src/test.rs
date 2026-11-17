#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, Home, HomeComponentValue, HomeLayout, MangaStatus, Viewer,
	alloc::{string::ToString, vec},
	imports::{
		defaults::{DefaultValue, defaults_set},
		html::Html,
	},
};
use aidoku_test::aidoku_test;
use home::ROWS;

const SERIES: &str = "ennead-v6u319j";
const ACTION: &str = "Action-1-action";

const SLIDE: &str = r#"<a href="https://mangavyvy.com/manga/example-series"><div class="slider-item">
<div class="img-manga"><img class="cover-adult" src="https://example.com/cover.png"></div>
<div><h1 class="title">Example Series</h1><p class="alternative-title">Another Name</p>
<p> <span class="pre-title">Authors</span><span class="space">:</span> First Writer , Second, Writer , First Writer , </p>
<p><span class="pre-title">Status</span><span class="space">:</span><span class="text-completed">Completed</span></p>
<p><span class="pre-title">View</span><span class="space">:</span>7,782,321</p>
<p><span class="pre-title">Genres</span><span class="space">:</span> <span class="badge label-badge">Manhwa</span> <span class="badge label-badge">Drama</span></p>
<div class="desc"> A plain summary. </div></div></div></a>"#;

const SERIES_PAGE: &str = r#"<div class="div-manga"><div class="row">
<div class="img-manga"><img class="" src="https://example.com/cover.png"></div>
<div class="col-md-7"><h1 class="title">Example Series</h1><p>Another Name</p>
<p> <span class="pre-title">Authors</span><span class="space">:</span> <a href="/author/a">Second, Writer</a>, <a href="/author/b">First Writer ,</a> </p>
<p> <span class="pre-title">Artists</span><span class="space">:</span> -- </p>
<p><span class="pre-title">Status</span><span class="space">:</span><span class="text-ongoing">Ongoing</span></p>
<p><span class="pre-title">Genres</span><span class="space">:</span> <a class="badge label-badge" href="/genre/manga">Manga</a> <a class="badge label-badge" href="/genre/ecchi">Ecchi</a></p>
</div></div><div class="summary"><p class="title">Summary</p><p class="content"> Two lines. </p></div></div>
<div class="list-group">
<a class="list-chapter" href="https://example.com/r?data=9" id="chapter-212"><span>Chapter 212 : Ch 212 : S2 Finale</span><span class="chapter-new">New</span><p>2 hour ago</p></a>
<a class="list-chapter" href="https://example.com/r?data=8" id="chapter-211"><span>Chapter 211 : Ch 211</span><p>5 minutes ago</p></a>
<a class="list-chapter" href="https://example.com/r?data=7" id="chapter-false"><span>Extra</span><p>Dec 09, 2025</p></a>
<a class="list-chapter" href="https://example.com/r?data=6" id="chapter-12"><span>Chapter 12 : 12 Days Later</span><p>Dec 08, 2025</p></a>
<a class="list-chapter" href="https://example.com/r?data=5" id="chapter-0"><span>Vol.15 Chapter 136: Hometown Song</span><p>Dec 07, 2025</p></a>
<a class="list-chapter" href="https://example.com/r?data=4" id="chapter-0"><span>Vol.2 Fifth Course: The Power of Sake</span><p>3 week ago</p></a>
<a class="list-chapter" href="https://example.com/r?data=3" id="chapter-false"><span>Extra</span><p>Dec 05, 2025</p></a>
<a class="list-chapter" href="https://example.com/r?data=2" id="chapter-0.75"><span>Chapter 0.75 : extra.75</span><p>Dec 04, 2025</p></a>
</div>"#;

// The runner keeps a value that is set to `Null`, so each default is written out.
fn reset() {
	exclude(&[]);
}

fn exclude(genres: &[&str]) {
	defaults_set(
		"excludedGenres",
		DefaultValue::StringArray(genres.iter().map(|genre| genre.to_string()).collect()),
	);
}

fn sort(index: i32, ascending: bool) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending,
	}
}

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
}

fn genres(included: &[&str], excluded: &[&str]) -> FilterValue {
	let owned = |ids: &[&str]| ids.iter().map(|id| id.to_string()).collect();
	FilterValue::MultiSelect {
		id: "genre".into(),
		included: owned(included),
		excluded: owned(excluded),
	}
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	VyManga
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn keys(page: &MangaPageResult) -> Vec<&str> {
	page.entries
		.iter()
		.map(|manga| manga.key.as_str())
		.collect()
}

fn series(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

fn update(key: &str, needs_details: bool, needs_chapters: bool) -> Manga {
	VyManga
		.get_manga_update(series(key), needs_details, needs_chapters)
		.unwrap()
}

#[aidoku_test]
fn review_home_rows_and_listings() {
	assert_eq!(
		ROWS,
		[
			("updated_at", "Latest Update"),
			("created_at", "New Release")
		]
	);
	let source: serde_json::Value =
		serde_json::from_str(include_str!("../res/source.json")).unwrap();
	let listings: Vec<&str> = source["listings"]
		.as_array()
		.unwrap()
		.iter()
		.map(|listing| listing["id"].as_str().unwrap())
		.collect();
	assert_eq!(listings, ROWS.map(|(sort, _)| sort));
	assert!(ROWS.iter().all(|(sort, _)| SORTS.contains(sort)));
}

#[aidoku_test]
fn review_slide_fills_the_big_scroller() {
	let html = Html::parse(SLIDE).unwrap();
	let slide = html.select_first("a").unwrap();
	let key = manga_key(&slide.attr("href").unwrap()).unwrap();
	let manga = parse_manga(key, &slide).unwrap();
	assert_eq!(manga.key, "example-series");
	assert_eq!(manga.title, "Example Series");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://example.com/cover.png")
	);
	assert_eq!(
		manga.authors,
		Some(vec!["First Writer".into(), "Second, Writer".into()])
	);
	assert_eq!(manga.artists, None);
	assert_eq!(manga.description.as_deref(), Some("A plain summary."));
	assert_eq!(manga.tags, Some(vec!["Manhwa".into(), "Drama".into()]));
	assert_eq!(manga.status, MangaStatus::Completed);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert!(manga.url.unwrap().ends_with("/manga/example-series"));
}

#[aidoku_test]
fn review_series_page_details() {
	let html = Html::parse(SERIES_PAGE).unwrap();
	let info = html.select_first(".div-manga").unwrap();
	let manga = parse_manga("example-series".into(), &info).unwrap();
	assert_eq!(manga.title, "Example Series");
	assert_eq!(
		manga.authors,
		Some(vec!["Second, Writer".into(), "First Writer".into()])
	);
	assert_eq!(manga.artists, None);
	assert_eq!(manga.description.as_deref(), Some("Two lines."));
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.content_rating, ContentRating::Suggestive);
	assert_eq!(manga.viewer, Viewer::RightToLeft);

	let untitled = Html::parse(r#"<div class="div-manga"><p>404</p></div>"#).unwrap();
	let info = untitled.select_first(".div-manga").unwrap();
	assert!(parse_manga("missing".into(), &info).is_err());
}

#[aidoku_test]
fn review_chapter_keys_numbers_and_titles() {
	let chapters = parse_chapters(&Html::parse(SERIES_PAGE).unwrap());
	let read = |index: usize| {
		let chapter = &chapters[index];
		(
			chapter.key.as_str(),
			chapter.volume_number,
			chapter.chapter_number,
			chapter.title.as_deref(),
		)
	};
	assert_eq!(chapters.len(), 8);
	assert_eq!(
		read(0),
		(
			"Chapter 212 : Ch 212 : S2 Finale",
			None,
			Some(212.0),
			Some("Ch 212 : S2 Finale")
		)
	);
	assert_eq!(read(1), ("Chapter 211 : Ch 211", None, Some(211.0), None));
	assert_eq!(read(2), ("Extra#2", None, None, Some("Extra")));
	assert_eq!(
		read(3),
		(
			"Chapter 12 : 12 Days Later",
			None,
			Some(12.0),
			Some("12 Days Later")
		)
	);
	assert_eq!(
		read(4),
		(
			"Vol.15 Chapter 136: Hometown Song",
			Some(15.0),
			Some(136.0),
			Some("Hometown Song")
		)
	);
	assert_eq!(
		read(5),
		(
			"Vol.2 Fifth Course: The Power of Sake",
			Some(2.0),
			None,
			Some("Fifth Course: The Power of Sake")
		)
	);
	assert_eq!(read(6), ("Extra", None, None, Some("Extra")));
	assert_eq!(
		read(7),
		(
			"Chapter 0.75 : extra.75",
			None,
			Some(0.75),
			Some("extra.75")
		)
	);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://example.com/r?data=9")
	);

	// The runner parses no date-only format, so only relative dates are checked here.
	let now = aidoku::imports::std::current_date();
	let age = |index: usize| now - chapters[index].date_uploaded.unwrap();
	assert!((7200..7260).contains(&age(0)));
	assert!((300..360).contains(&age(1)));
	assert_eq!(chapters[5].date_uploaded, None);
}

#[aidoku_test]
fn review_search_url_applies_settings() {
	exclude(&["Yaoi-36-yaoi", ACTION]);
	let url = sorted_url("updated_at", 2);
	assert!(url.contains("/search?sort=updated_at&page=2&"));
	assert_eq!(url.matches("exclude_genre%5B%5D=").count(), 2);
	assert!(url.contains("exclude_genre%5B%5D=Action-1-action"));

	// Including a genre in a search lifts its exclusion in settings.
	let url = search_url(
		QueryParameters::new(),
		1,
		&[ACTION.into()],
		vec!["Drama-4-drama".into()],
	);
	assert!(url.contains("genre%5B%5D=Action-1-action"));
	assert!(!url.contains("exclude_genre%5B%5D=Action-1-action"));
	assert!(url.contains("exclude_genre%5B%5D=Yaoi-36-yaoi"));
	assert!(url.contains("exclude_genre%5B%5D=Drama-4-drama"));

	reset();
	assert!(!sorted_url("created_at", 1).contains("exclude_genre"));
}

#[aidoku_test]
fn review_series_links() {
	for (url, key) in [
		(
			"https://mangavyvy.com/manga/ennead-v6u319j",
			Some("ennead-v6u319j"),
		),
		("https://mangavyvy.net/manga/oishinbo/", Some("oishinbo")),
		(
			"/manga/this-cant-be-love+++?ref=home#top",
			Some("this-cant-be-love+++"),
		),
		("https://mangavyvy.com/manga/", None),
		("https://mangavyvy.com/genre/action", None),
	] {
		assert_eq!(manga_key(url).as_deref(), key, "{url}");
	}
	assert_eq!(
		VyManga
			.handle_deep_link("https://mangavyvy.net/manga/oishinbo".into())
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "oishinbo".into()
		})
	);
	assert_eq!(
		VyManga
			.handle_deep_link("https://mangavyvy.com/search".into())
			.unwrap(),
		None
	);
}

#[aidoku_test]
fn review_genre_lists_match() {
	let filters: serde_json::Value =
		serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let settings: serde_json::Value =
		serde_json::from_str(include_str!("../res/settings.json")).unwrap();
	let genre = filters
		.as_array()
		.unwrap()
		.iter()
		.find(|filter| filter["id"] == "genre")
		.unwrap();
	let setting = &settings[0]["items"][0];
	assert_eq!(setting["key"], "excludedGenres");
	assert_eq!(genre["ids"], setting["values"]);
	assert_eq!(genre["options"], setting["titles"]);
	assert_eq!(genre["canExclude"], true);
	assert_eq!(genre["isGenre"], true);
	let ids = genre["ids"].as_array().unwrap();
	assert_eq!(ids.len(), genre["options"].as_array().unwrap().len());
	assert!(ids.iter().any(|id| id == ACTION));
	assert_eq!(filters[0]["type"], "sort");
	assert_eq!(filters[0]["options"].as_array().unwrap().len(), SORTS.len());
}

#[aidoku_test]
fn browse_pages_differ() {
	reset();
	let first = search(None, Vec::new());
	let second = VyManga.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert_eq!(first.entries.len(), 36);
	assert!(first.has_next_page);
	let mut unique = keys(&first);
	unique.sort_unstable();
	unique.dedup();
	assert_eq!(unique.len(), 36);
	assert!(keys(&second).iter().all(|key| !keys(&first).contains(key)));
	assert!(
		first
			.entries
			.iter()
			.all(|manga| !manga.title.is_empty() && manga.cover.is_some())
	);
	assert!(first.entries.iter().all(|manga| manga.authors.is_none()
		&& manga.description.is_none()
		&& manga.tags.is_none()));
}

#[aidoku_test]
fn every_sort_and_direction_changes_the_order() {
	reset();
	let mut seen: Vec<Vec<String>> = Vec::new();
	for index in 0..4 {
		for ascending in [false, true] {
			let page = search(None, vec![sort(index, ascending)]);
			assert!(
				page.entries.len() > 30,
				"sort {index} ascending {ascending}"
			);
			let order: Vec<String> = page.entries.into_iter().map(|manga| manga.key).collect();
			assert!(!seen.contains(&order), "sort {index} ascending {ascending}");
			seen.push(order);
		}
	}
	// Viewed, descending, is the site's own default.
	assert_eq!(keys(&search(None, Vec::new())), seen[0]);
}

#[aidoku_test]
fn status_filter_matches_the_cards() {
	reset();
	let completed = search(None, vec![select("status", "1")]);
	let ongoing = search(None, vec![select("status", "0")]);
	assert!(!completed.entries.is_empty() && !ongoing.entries.is_empty());
	assert!(
		completed
			.entries
			.iter()
			.all(|manga| manga.status == MangaStatus::Completed)
	);
	assert!(
		ongoing
			.entries
			.iter()
			.all(|manga| manga.status == MangaStatus::Ongoing)
	);
}

#[aidoku_test]
fn text_search_finds_a_series() {
	reset();
	let found = search(Some("Oishinbo"), Vec::new());
	assert!(keys(&found).contains(&"oishinbo"));
	let none = search(Some("zzzzqqqqxxxx"), Vec::new());
	assert!(none.entries.is_empty() && !none.has_next_page);
}

#[aidoku_test]
fn description_search_widens_the_results() {
	reset();
	let titles = search(Some("pirate"), Vec::new());
	let described = search(
		Some("pirate"),
		vec![FilterValue::Check {
			id: "description".into(),
			value: 1,
		}],
	);
	assert!(!titles.has_next_page);
	assert!(described.has_next_page);
}

#[aidoku_test]
fn author_filter_and_its_match() {
	reset();
	let author = |name: &str| FilterValue::Text {
		id: "author".into(),
		value: name.into(),
	};
	let exact = search(None, vec![author("Kariya, Tetsu")]);
	assert!(keys(&exact).contains(&"oishinbo"));
	let contain = search(None, vec![author("oda")]);
	let begin = search(None, vec![author("oda"), select("author_match", "1")]);
	let end = search(None, vec![author("oda"), select("author_match", "2")]);
	assert_ne!(keys(&contain), keys(&begin));
	assert_ne!(keys(&contain), keys(&end));
	assert_ne!(keys(&begin), keys(&end));
	assert!(
		search(None, vec![author("zzzzqqqqxxxx")])
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn genre_filter_includes_and_excludes() {
	reset();
	let default = search(None, Vec::new());
	let action = search(None, vec![genres(&[ACTION], &[])]);
	assert!(!action.entries.is_empty());
	assert_ne!(keys(&action), keys(&default));
	let tags = update(&action.entries[0].key, true, false).tags.unwrap();
	assert!(tags.iter().any(|tag| tag == "Action"), "{tags:?}");

	let without = search(None, vec![genres(&[], &[ACTION])]);
	assert!(
		keys(&without)
			.iter()
			.all(|key| !keys(&action).contains(key))
	);
	assert!(
		search(None, vec![genres(&["zzz-99999-zzz"], &[])])
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn excluded_genres_setting_and_its_override() {
	reset();
	let without = search(None, vec![genres(&[], &[ACTION])]);
	let action = search(None, vec![genres(&[ACTION], &[])]);
	exclude(&[ACTION]);
	assert_eq!(keys(&search(None, Vec::new())), keys(&without));
	let listing = VyManga
		.get_manga_list(
			Listing {
				id: "updated_at".into(),
				..Default::default()
			},
			1,
		)
		.unwrap();
	assert!(!listing.entries.is_empty());
	assert_eq!(
		keys(&listing),
		keys(&search(None, vec![sort(3, false), genres(&[], &[ACTION])]))
	);
	// The site answers nothing when one genre is both included and excluded.
	assert_eq!(
		keys(&search(None, vec![genres(&[ACTION], &[])])),
		keys(&action)
	);
	reset();
}

#[aidoku_test]
fn listings_are_the_sorted_search() {
	reset();
	for (index, (id, _)) in [(3, ROWS[0]), (2, ROWS[1])] {
		let listing = VyManga
			.get_manga_list(
				Listing {
					id: id.into(),
					..Default::default()
				},
				1,
			)
			.unwrap();
		assert_eq!(
			keys(&listing),
			keys(&search(None, vec![sort(index, false)]))
		);
	}
	let unknown = Listing {
		id: "unknown".into(),
		..Default::default()
	};
	assert!(VyManga.get_manga_list(unknown, 1).is_err());
}

#[aidoku_test]
fn details_and_chapters() {
	reset();
	let manga = update(SERIES, true, true);
	assert_eq!(manga.key, SERIES);
	assert_eq!(manga.title, "Ennead");
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.authors, None);
	assert!(manga.cover.unwrap().starts_with("https://"));
	assert!(manga.description.unwrap().contains("Egypt"));
	assert!(manga.url.unwrap().ends_with("/manga/ennead-v6u319j"));
	let tags = manga.tags.unwrap();
	assert!(tags.iter().any(|tag| tag == "Yaoi"), "{tags:?}");

	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() > 200);
	let mut unique: Vec<&str> = chapters
		.iter()
		.map(|chapter| chapter.key.as_str())
		.collect();
	unique.sort_unstable();
	unique.dedup();
	assert_eq!(unique.len(), chapters.len());
	let finale = chapters
		.iter()
		.find(|chapter| chapter.key == "Chapter 212 : Ch 212 : S2 Finale")
		.unwrap();
	assert_eq!(finale.chapter_number, Some(212.0));
	assert_eq!(finale.title.as_deref(), Some("Ch 212 : S2 Finale"));
	assert!(chapters.iter().any(|chapter| chapter.key == "Extra#2"));
	assert!(chapters.iter().all(|chapter| chapter.url.is_some()));
}

#[aidoku_test]
fn chapter_keys_outlive_their_links() {
	reset();
	let first = update(SERIES, false, true).chapters.unwrap();
	let second = update(SERIES, false, true).chapters.unwrap();
	let read = |chapters: &[Chapter]| -> Vec<String> {
		chapters.iter().map(|chapter| chapter.key.clone()).collect()
	};
	assert_eq!(read(&first), read(&second));
	assert_ne!(first[0].url, second[0].url);
}

#[aidoku_test]
fn chapters_only_update_keeps_the_details() {
	reset();
	let manga = VyManga
		.get_manga_update(
			Manga {
				key: SERIES.into(),
				title: "Kept".into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	assert_eq!(manga.title, "Kept");
	assert!(manga.chapters.is_some());
}

#[aidoku_test]
fn pages_from_keys_alone() {
	reset();
	let chapter = Chapter {
		key: "Chapter 1".into(),
		..Default::default()
	};
	let pages = VyManga.get_page_list(series(SERIES), chapter).unwrap();
	assert!(pages.len() > 5);
	assert!(pages.iter().all(|page| match &page.content {
		PageContent::Url(url, _) => url.starts_with("https://"),
		_ => false,
	}));
}

#[aidoku_test]
fn unknown_keys_are_errors() {
	reset();
	let missing = Chapter {
		key: "Chapter 99999".into(),
		..Default::default()
	};
	let error = VyManga.get_page_list(series(SERIES), missing).unwrap_err();
	assert!(
		format!("{error:?}").contains("Chapter not found"),
		"{error:?}"
	);
	let error = VyManga
		.get_manga_update(series("zzzz-not-a-series-zz"), true, true)
		.unwrap_err();
	assert!(format!("{error:?}").contains("404"), "{error:?}");
}

#[aidoku_test]
fn home_follows_the_site() {
	reset();
	let components = VyManga.get_home().unwrap().components;
	let titles: Vec<&str> = components
		.iter()
		.map(|component| component.title.as_deref().unwrap())
		.collect();
	assert_eq!(titles, ["Popular Manga", "Latest Update", "New Release"]);

	let HomeComponentValue::BigScroller { entries, .. } = &components[0].value else {
		panic!("Popular Manga is not a big scroller");
	};
	assert_eq!(entries.len(), 16);
	assert!(entries.iter().all(|manga| {
		!manga.key.is_empty()
			&& !manga.title.is_empty()
			&& manga.cover.is_some()
			&& manga.tags.is_some()
			&& manga.status != MangaStatus::Unknown
			&& manga.content_rating != ContentRating::Unknown
	}));
	assert!(entries.iter().any(|manga| manga.authors.is_some()));
	assert!(
		entries
			.iter()
			.filter(|manga| manga.description.is_some())
			.count() > 12
	);

	for (index, (sort, title)) in ROWS.iter().enumerate() {
		let HomeComponentValue::Scroller { entries, listing } = &components[index + 1].value else {
			panic!("{title} is not a scroller");
		};
		assert!(entries.len() > 30);
		assert!(entries.iter().all(|link| link.subtitle.is_none()));
		assert_eq!(listing.as_ref().unwrap().id, *sort);
	}
}

#[aidoku_test]
fn home_hides_excluded_genres() {
	reset();
	let popular = |home: HomeLayout| match home.components.into_iter().next().unwrap().value {
		HomeComponentValue::BigScroller { entries, .. } => entries,
		_ => panic!("Popular Manga is not a big scroller"),
	};
	let all = popular(VyManga.get_home().unwrap());
	let tag = all[0].tags.as_ref().unwrap()[0].clone();
	let filters: serde_json::Value =
		serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let genre = filters
		.as_array()
		.unwrap()
		.iter()
		.find(|filter| filter["id"] == "genre")
		.unwrap();
	let position = genre["options"]
		.as_array()
		.unwrap()
		.iter()
		.position(|option| option == tag.as_str())
		.unwrap();
	exclude(&[genre["ids"][position].as_str().unwrap()]);
	let shown = popular(VyManga.get_home().unwrap());
	reset();
	assert!(shown.len() < all.len());
	assert!(
		shown
			.iter()
			.all(|manga| !manga.tags.as_ref().unwrap().contains(&tag))
	);
}
