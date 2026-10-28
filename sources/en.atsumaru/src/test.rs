#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, DynamicSettings, Home, HomeComponentValue, MangaStatus, Viewer,
	alloc::{format, string::ToString},
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use api::SearchParams;
use serde::Deserialize;

const COMIC: &str = "n86El";
const NOVEL: &str = "novel/slh2";
// Has the Adult and Smut genres but no rating from the site.
const UNRATED_ADULT: &str = "novel/XyKI";

#[derive(Deserialize)]
struct Found {
	hits: Vec<Hit>,
}

#[derive(Deserialize)]
struct Hit {
	document: Doc,
}

// Verify against an independent lookup.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Doc {
	id: String,
	#[serde(default)]
	is_adult: bool,
	mb_content_rating: Option<String>,
	#[serde(default)]
	genre_ids: Vec<String>,
	#[serde(default)]
	tag_ids: Vec<String>,
	medium: Option<String>,
	#[serde(rename = "type")]
	kind: Option<String>,
	#[serde(default)]
	official_translation: bool,
}

impl Doc {
	fn has_genre(&self, id: &str) -> bool {
		self.genre_ids.iter().any(|genre| genre == id)
	}

	fn has_tag(&self, id: &str) -> bool {
		self.tag_ids.iter().any(|tag| tag == id)
	}

	fn is_novel(&self) -> bool {
		self.medium.as_deref() == Some("Novel")
	}

	fn tier(&self) -> &'static str {
		let rating = self.mb_content_rating.as_deref();
		if self.is_adult || rating == Some("Pornographic") || self.has_genre("10") {
			"Pornographic"
		} else if rating == Some("Erotica") || self.has_genre("46") || self.has_genre("41") {
			"Erotica"
		} else if rating == Some("Suggestive") {
			"Suggestive"
		} else {
			"Safe"
		}
	}
}

fn docs(result: &MangaPageResult) -> Vec<Doc> {
	if result.entries.is_empty() {
		return Vec::new();
	}
	let ids = result
		.entries
		.iter()
		.map(|manga| manga_id(&manga.key))
		.collect::<Vec<_>>();
	let url = api::lookup_url(
		&ids,
		"id,isAdult,mbContentRating,genreIds,tagIds,medium,type,officialTranslation",
	);
	get::<Found>(&url)
		.unwrap()
		.hits
		.into_iter()
		.map(|hit| hit.document)
		.collect()
}

fn keys(result: &MangaPageResult) -> Vec<String> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.clone())
		.collect()
}

fn assert_compact(result: &MangaPageResult) {
	for manga in &result.entries {
		assert!(
			manga.description.is_none()
				&& manga.tags.is_none()
				&& manga.authors.is_none()
				&& manga.artists.is_none()
				&& manga.url.is_none(),
			"{}",
			manga.key
		);
	}
}

fn strings(values: &[&str]) -> DefaultValue {
	DefaultValue::StringArray(values.iter().map(|value| (*value).into()).collect())
}

fn clear(keys: &[&str]) {
	for key in keys {
		defaults_set(key, DefaultValue::Null);
	}
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	Atsumaru::new()
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: Vec::new(),
	}
}

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
}

fn sort(index: i32, ascending: bool) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending,
	}
}

fn listing(id: &str, page: i32) -> MangaPageResult {
	Atsumaru::new()
		.get_manga_list(
			Listing {
				id: id.into(),
				..Default::default()
			},
			page,
		)
		.unwrap()
}

fn manga(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

#[aidoku_test]
fn browse_pages() {
	let first = search(None, Vec::new());
	let second = Atsumaru::new()
		.get_search_manga_list(None, 2, Vec::new())
		.unwrap();
	assert_eq!(first.entries.len(), 40);
	assert!(first.has_next_page);
	assert_compact(&first);
	assert_ne!(keys(&first), keys(&second));
	for doc in docs(&first) {
		assert_ne!(doc.tier(), "Pornographic", "{}", doc.id);
	}
}

#[aidoku_test]
fn text_search() {
	for query in [
		"stellar swordmaster",
		"re:zero",
		"별을 품은",
		"solo-leveling & more",
	] {
		search(Some(query), Vec::new());
	}
	assert!(keys(&search(Some("stellar swordmaster"), Vec::new())).contains(&COMIC.to_string()));
	assert!(
		search(Some("qqqqqqqqqqqqqqxj"), Vec::new())
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn pasted_links() {
	for url in [
		"https://atsu.moe/manga/n86El",
		"https://atsu.moe/read/n86El/3uheUh",
		" https://atsu.moe/manga/n86El ",
	] {
		let result = search(Some(url), Vec::new());
		assert_eq!(keys(&result), [COMIC.to_string()], "{url}");
		assert_compact(&result);
	}
	assert_eq!(
		keys(&search(Some("https://atsu.moe/novel/slh2"), Vec::new())),
		[NOVEL.to_string()]
	);
	// Unknown series links fall back to text search.
	search(Some("https://atsu.moe/manga/zzzzzz"), Vec::new());
}

#[aidoku_test]
fn sorts() {
	for index in 0..16 {
		assert!(
			!search(None, vec![sort(index, false)]).entries.is_empty(),
			"{index}"
		);
	}
	// Site collation can differ from Rust string order.
	let titled = keys(&search(None, vec![sort(1, false)]));
	assert_eq!(titled, keys(&search(None, vec![sort(1, false)])));
	assert_ne!(titled, keys(&search(None, vec![sort(0, false)])));
	assert_ne!(
		keys(&search(Some("sword"), Vec::new())),
		keys(&search(Some("sword"), vec![sort(1, false)]))
	);
}

#[aidoku_test]
fn sort_directions() {
	let sorted = |index: i32, ascending: bool| search(None, vec![sort(index, ascending)]);
	assert_ne!(keys(&sorted(0, false)), keys(&sorted(0, true)));
	let from_z = sorted(1, false).entries[0].title.to_lowercase();
	let from_a = sorted(1, true).entries[0].title.to_lowercase();
	assert!(from_z.starts_with('z'), "{from_z}");
	assert!(!from_a.starts_with('z'), "{from_a}");
	// Feeds have no reverse order.
	assert_eq!(keys(&sorted(8, false)), keys(&sorted(8, true)));
}

#[aidoku_test]
fn timeframes() {
	let sorts = [
		("popular", 0),
		("mostBookmarked", 10),
		("mostTalkedAbout", 11),
	];
	let before = sorts
		.iter()
		.map(|(_, index)| keys(&search(None, vec![sort(*index, false)])))
		.collect::<Vec<_>>();
	for (id, _) in sorts {
		defaults_set(
			&format!("{id}Timeframe"),
			DefaultValue::String("monthly".into()),
		);
	}
	for ((_, index), expected) in sorts.iter().zip(before) {
		assert_eq!(keys(&search(None, vec![sort(*index, false)])), expected);
	}
	defaults_set(
		"homeSections",
		strings(&["popular", "mostBookmarked", "mostTalkedAbout"]),
	);
	let home = Atsumaru::new().get_home().unwrap();
	assert_eq!(home.components.len(), 3);
	for component in home.components {
		assert!(component.title.unwrap().ends_with("This Month"));
	}
	for (id, _) in sorts {
		let params = SearchParams::new(None);
		let expected: models::FeedResponse =
			api::get(&params.feed_url(id, 1, Some("monthly")).unwrap()).unwrap();
		assert_eq!(
			manga_id(&listing(id, 1).entries[0].key),
			expected.items[0].id
		);
		defaults_set(&format!("{id}Timeframe"), DefaultValue::Null);
	}
	clear(&["homeSections"]);
}

#[aidoku_test]
fn feed_sorts() {
	// Rising is the third feed sort: the feed's order, narrowed by the search filters.
	let feed = keys(&listing("rising", 1));
	let narrowed = search(None, vec![sort(8, false), multi("genre", &["39"])]);
	assert!(!narrowed.entries.is_empty());
	assert!(docs(&narrowed).iter().all(|doc| doc.has_genre("39")));
	let mut position = 0;
	for key in keys(&narrowed) {
		position += feed[position..]
			.iter()
			.position(|feed_key| *feed_key == key)
			.unwrap() + 1;
	}
}

#[aidoku_test]
fn random_sort() {
	let shuffled = |seed: u32| {
		let mut params = SearchParams::new(None);
		params.sort = 15;
		params.seed = seed;
		keys(&params.page(1, false).unwrap())
	};
	assert_eq!(shuffled(7), shuffled(7));
	assert_ne!(shuffled(7), shuffled(8));
}

#[aidoku_test]
fn genres() {
	let all = search(None, vec![multi("genre", &["39", "9"])]);
	let any = search(
		None,
		vec![multi("genre", &["39", "9"]), select("genre_match", "any")],
	);
	assert_ne!(keys(&all), keys(&any));
	for doc in docs(&all) {
		assert!(doc.has_genre("39") && doc.has_genre("9"), "{}", doc.id);
	}
}

#[aidoku_test]
fn tapped_tags() {
	// Martial Arts is a genre, Isekai a tag; names arrive in any case.
	let martial = search(None, vec![select("genre", "martial arts")]);
	let isekai = search(None, vec![select("genre", "Isekai")]);
	assert!(!martial.entries.is_empty() && !isekai.entries.is_empty());
	assert!(docs(&martial).iter().all(|doc| doc.has_genre("29")));
	assert!(docs(&isekai).iter().all(|doc| doc.has_tag("94")));
}

#[aidoku_test]
fn other_filters() {
	let author = |name: &str| {
		search(
			None,
			vec![FilterValue::Text {
				id: "author".into(),
				value: name.into(),
			}],
		)
	};
	assert!(keys(&author("Hong Daeui")).contains(&COMIC.to_string()));
	assert!(author("zzqq nobody`\\").entries.is_empty());
	let ranged = search(
		None,
		vec![
			FilterValue::Range {
				id: "year".into(),
				from: Some(2020.0),
				to: Some(2021.0),
			},
			FilterValue::Range {
				id: "chapters".into(),
				from: Some(100.0),
				to: None,
			},
		],
	);
	assert!(!ranged.entries.is_empty());
	let official = search(
		None,
		vec![FilterValue::Check {
			id: "official".into(),
			value: 1,
		}],
	);
	assert!(!official.entries.is_empty());
	assert!(docs(&official).iter().all(|doc| doc.official_translation));
}

#[aidoku_test]
fn content_ratings() {
	for rating in ["Safe", "Suggestive", "Erotica", "Pornographic"] {
		defaults_set("contentRatings", strings(&[rating]));
		let searched = search(None, Vec::new());
		let feed = listing("popular", 1);
		clear(&["contentRatings"]);
		assert!(!searched.entries.is_empty(), "{rating}");
		for doc in docs(&searched) {
			assert_eq!(doc.tier(), rating, "search {}", doc.id);
		}
		// Feeds cannot distinguish unrated titles.
		for doc in docs(&feed) {
			let tier = doc.tier();
			assert!(tier == rating || tier == "Safe", "feed {} {tier}", doc.id);
		}
	}
	let picked = search(None, vec![multi("rating", &["Pornographic"])]);
	assert!(!picked.entries.is_empty());
	assert!(docs(&picked).iter().all(|doc| doc.tier() == "Pornographic"));
	defaults_set("contentRatings", strings(&[]));
	let searched = search(None, Vec::new());
	let feed = listing("popular", 1);
	clear(&["contentRatings"]);
	assert!(searched.entries.is_empty() && feed.entries.is_empty());
}

#[aidoku_test]
fn unrated_titles_rated_from_genres() {
	let found =
		|| keys(&search(Some("SON OF THE HERO KING"), Vec::new())).contains(&UNRATED_ADULT.into());
	assert!(found());
	defaults_set("contentRatings", strings(&["Safe", "Suggestive"]));
	let hidden = !found();
	clear(&["contentRatings"]);
	assert!(hidden);
	// Hentai needs Pornographic, which is off by default.
	assert!(
		search(None, vec![multi("genre", &["10"])])
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn adult_only() {
	defaults_set("adultOnly", DefaultValue::Bool(true));
	let searched = search(None, Vec::new());
	let feed = listing("popular", 1);
	let picked = search(None, vec![multi("rating", &["Safe"])]);
	clear(&["adultOnly"]);
	assert!(!searched.entries.is_empty() && !feed.entries.is_empty());
	for doc in docs(&searched).iter().chain(&docs(&feed)) {
		assert_eq!(doc.tier(), "Pornographic", "{}", doc.id);
	}
	assert!(!picked.entries.is_empty());
	assert!(docs(&picked).iter().all(|doc| doc.tier() == "Safe"));
}

#[aidoku_test]
fn content_types() {
	let under = |types: &[&str], results: fn() -> Vec<MangaPageResult>| {
		defaults_set("contentTypes", strings(types));
		let results = results();
		clear(&["contentTypes"]);
		results
	};
	let novels = under(&["Novel"], || {
		vec![
			search(None, Vec::new()),
			listing("popular", 1),
			listing("rising", 1),
		]
	});
	for result in &novels {
		assert!(!result.entries.is_empty());
		assert!(docs(result).iter().all(Doc::is_novel));
	}
	let manga = under(&["Manga"], || {
		vec![search(None, Vec::new()), listing("popular", 1)]
	});
	for result in &manga {
		assert!(!result.entries.is_empty());
		assert!(
			docs(result)
				.iter()
				.all(|doc| !doc.is_novel() && doc.kind.as_deref() == Some("Manga"))
		);
	}
	let mixed = under(&["Manwha", "Novel"], || {
		vec![search(None, Vec::new()), listing("hotUpdates", 1)]
	});
	for result in &mixed {
		assert!(
			docs(result)
				.iter()
				.all(|doc| doc.is_novel() || doc.kind.as_deref() == Some("Manwha"))
		);
	}
	let none = under(&[], || {
		vec![
			search(None, Vec::new()),
			listing("popular", 1),
			listing("novels", 1),
		]
	});
	assert!(none.iter().all(|result| result.entries.is_empty()));
	let picked = under(&["Novel"], || {
		vec![search(None, vec![multi("type", &["Manga"])])]
	});
	assert!(!picked[0].entries.is_empty());
	assert!(
		docs(&picked[0])
			.iter()
			.all(|doc| !doc.is_novel() && doc.kind.as_deref() == Some("Manga"))
	);
}

#[aidoku_test]
fn exclusions() {
	assert_eq!(Atsumaru::new().get_dynamic_settings().unwrap().len(), 1);
	defaults_set("hiddenGenres", strings(&["39"]));
	defaults_set("hiddenTags.Settings", strings(&["94"]));
	let results = [
		search(None, Vec::new()),
		listing("popular", 1),
		listing("recentlyUpdated", 1),
		listing("novels", 1),
	];
	let asked = search(None, vec![multi("genre", &["39"])]);
	clear(&["hiddenGenres", "hiddenTags.Settings"]);
	for result in &results {
		assert!(!result.entries.is_empty());
		for doc in docs(result) {
			assert!(!doc.has_genre("39") && !doc.has_tag("94"), "{}", doc.id);
		}
	}
	assert!(!asked.entries.is_empty());
	assert!(docs(&asked).iter().all(|doc| doc.has_genre("39")));
}

#[aidoku_test]
fn filter_exclusions() {
	for (id, excluded) in [("genre", "39"), ("demographic", "38")] {
		let result = search(
			None,
			vec![FilterValue::MultiSelect {
				id: id.into(),
				included: Vec::new(),
				excluded: vec![excluded.into()],
			}],
		);
		assert!(!result.entries.is_empty(), "{id}");
		for doc in docs(&result) {
			assert!(if id == "genre" {
				!doc.has_genre(excluded)
			} else {
				!doc.has_tag(excluded)
			});
		}
	}
}

#[aidoku_test]
fn review_filter_contract() {
	#[derive(Deserialize)]
	#[serde(rename_all = "camelCase")]
	struct Filter {
		id: String,
		#[serde(default)]
		can_exclude: bool,
	}
	let filters: Vec<Filter> = serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let excluded = filters
		.iter()
		.filter(|filter| filter.can_exclude)
		.map(|filter| filter.id.as_str())
		.collect::<Vec<_>>();
	assert_eq!(excluded, ["genre", "demographic"]);
}

#[aidoku_test]
fn review_series_keys() {
	let item = || serde_json::from_str::<models::ChapterItem>(r#"{"id":"5nDftC"}"#).unwrap();
	let novel = item().into_chapter(manga_id(NOVEL), true, &[]);
	assert_eq!(novel.key, "5nDftC");
	assert_eq!(
		novel.url.as_deref(),
		Some("https://atsu.moe/novel/slh2/5nDftC")
	);
	assert_eq!(
		helpers::parse_link(novel.url.as_deref().unwrap()),
		Some((NOVEL.into(), Some(novel.key)))
	);
	let comic = item().into_chapter(COMIC, false, &[]);
	assert_eq!(comic.key, "5nDftC");
	assert_eq!(
		comic.url.as_deref(),
		Some("https://atsu.moe/read/n86El/5nDftC")
	);
	assert_eq!(
		helpers::parse_link(comic.url.as_deref().unwrap()),
		Some((COMIC.into(), Some(comic.key)))
	);
}

#[aidoku_test]
fn review_series_key_conversions() {
	for (medium, expected) in [("Novel", "novel/slh2"), ("Comic", "slh2")] {
		let json = format!(r#"{{"id":"slh2","title":"Title","medium":"{medium}"}}"#);
		let feed: models::FeedItem = serde_json::from_str(&json).unwrap();
		let search: models::SearchDocument = serde_json::from_str(&json).unwrap();
		let details: models::MangaInfo = serde_json::from_str(&json).unwrap();
		assert_eq!(feed.into_manga(false).key, expected);
		assert_eq!(search.into_manga(false).key, expected);
		assert_eq!(details.into_manga(false, None).key, expected);
		assert_eq!(manga_id(expected), "slh2");
	}
}

#[aidoku_test]
fn review_featured_covers() {
	for (images, card, featured) in [
		(
			r#""image":"small.avif","mediumImage":"medium.avif","largeImage":"large.avif""#,
			Some("medium.webp"),
			Some("large.webp"),
		),
		(
			r#""image":"small.avif","mediumImage":"medium.avif""#,
			Some("medium.webp"),
			Some("medium.webp"),
		),
		(
			r#""image":"small.avif""#,
			Some("small.webp"),
			Some("small.webp"),
		),
		(r#""largeImage":"large.avif""#, None, Some("large.webp")),
		(r#""image":null"#, None, None),
	] {
		let json = format!(r#"{{"id":"id","title":"Title",{images}}}"#);
		let item = || serde_json::from_str::<models::FeedItem>(&json).unwrap();
		let expected = |path: Option<&str>| path.map(helpers::cover_url);
		assert_eq!(item().into_manga(false).cover, expected(card));
		assert_eq!(item().into_featured(false, None).cover, expected(featured));
		let document =
			serde_json::from_str(r#"{"id":"id","title":"Title","synopsis":"Description"}"#)
				.unwrap();
		let enriched = item().into_featured(false, Some(document));
		assert_eq!(enriched.cover, expected(featured));
		assert_eq!(enriched.description.as_deref(), Some("Description"));
	}
}

#[aidoku_test]
fn listings() {
	for id in [
		"popular",
		"recentlyUpdated",
		"hotUpdates",
		"rising",
		"topRated",
		"genreSpotlight",
		"hotArrivals",
		"mostBookmarked",
		"mostTalkedAbout",
		"novels",
		"recentlyAdded",
		"hiddenGems",
		"bingeWorthy",
		"mostPolarizing",
	] {
		let first = listing(id, 1);
		assert!(!first.entries.is_empty() && first.has_next_page, "{id}");
		assert_compact(&first);
		assert_ne!(keys(&first), keys(&listing(id, 2)), "{id}");
	}
	assert!(
		Atsumaru::new()
			.get_manga_list(
				Listing {
					id: "nope".into(),
					..Default::default()
				},
				1
			)
			.is_err()
	);
}

#[aidoku_test]
fn home() {
	let home = Atsumaru::new().get_home().unwrap();
	assert_eq!(home.components.len(), 14);
	for component in &home.components {
		let rows = match &component.value {
			HomeComponentValue::BigScroller { entries, .. } => {
				assert!(entries.iter().any(|manga| manga.description.is_some()));
				entries.len()
			}
			HomeComponentValue::Scroller { entries, .. }
			| HomeComponentValue::MangaList { entries, .. } => {
				assert!(
					entries.iter().all(|link| link.subtitle.is_none()),
					"{:?}",
					component.title
				);
				entries.len()
			}
			_ => panic!("unexpected component {:?}", component.title),
		};
		assert!(rows > 0, "{:?} is empty", component.title);
	}
	defaults_set("homeSections", strings(&["rising"]));
	let one = Atsumaru::new().get_home().unwrap();
	defaults_set("homeSections", strings(&[]));
	let none = Atsumaru::new().get_home().unwrap();
	clear(&["homeSections"]);
	assert_eq!(one.components.len(), 1);
	assert!(none.components.is_empty());
}

#[aidoku_test]
fn comic() {
	let source = Atsumaru::new();
	let details = source.get_manga_update(manga(COMIC), true, false).unwrap();
	assert_eq!(details.title, "The Stellar Swordmaster");
	assert_eq!(details.url.as_deref(), Some("https://atsu.moe/manga/n86El"));
	assert_eq!(details.content_rating, ContentRating::Safe);
	assert_eq!(details.status, MangaStatus::Ongoing);
	assert_eq!(details.viewer, Viewer::Webtoon);
	assert!(details.authors.is_some() && details.artists.is_some());
	assert!(details.description.is_some() && details.chapters.is_none());

	let updated = source.get_manga_update(manga(COMIC), false, true).unwrap();
	assert!(updated.title.is_empty());
	let chapters = updated.chapters.unwrap();
	assert!(chapters.len() > 200);
	for chapter in &chapters {
		let url = chapter.url.as_deref().unwrap();
		assert!(url.starts_with("https://atsu.moe/read/n86El/") && chapter.language.is_none());
	}

	let pages = source
		.get_page_list(
			manga(COMIC),
			Chapter {
				key: chapters.into_iter().next().unwrap().key,
				..Default::default()
			},
		)
		.unwrap();
	assert!(!pages.is_empty());
	for page in pages {
		assert!(
			matches!(page.content, PageContent::Url(url, _) if url.starts_with("https://cdn.atsu.moe/static/pages/"))
		);
	}
}

fn chapter_fixtures() -> (Vec<models::ChapterItem>, Vec<models::Scanlator>) {
	let chapters = serde_json::from_str(
		r#"[
		{"id":"a-2a","scanlationMangaId":"a","number":2,"createdAt":10,"pageCount":10},
		{"id":"a-2b","scanlationMangaId":"a","number":2,"createdAt":20,"pageCount":15},
		{"id":"b-2","scanlationMangaId":"b","number":2,"createdAt":30,"pageCount":20},
		{"id":"a-1","scanlationMangaId":"a","number":1,"createdAt":100,"pageCount":20},
		{"id":"b-1","scanlationMangaId":"b","number":1,"createdAt":50,"pageCount":30},
		{"id":"a-special","scanlationMangaId":"a","createdAt":40,"pageCount":1},
		{"id":"b-special","scanlationMangaId":"b","createdAt":40,"pageCount":1}
	]"#,
	)
	.unwrap();
	let groups = serde_json::from_str(
		r#"[
		{"id":"a","name":"A","score":8},
		{"id":"b","name":"B","score":5}
	]"#,
	)
	.unwrap();
	(chapters, groups)
}

#[aidoku_test]
fn review_deduplication() {
	for (keep, expected) in [
		(
			settings::Keep::HighestRated,
			vec!["a-2b", "a-2a", "a-1", "a-special", "b-special"],
		),
		(
			settings::Keep::Newest,
			vec!["b-2", "a-1", "a-special", "b-special"],
		),
		(
			settings::Keep::MostPages,
			vec!["b-2", "b-1", "a-special", "b-special"],
		),
	] {
		let (chapters, groups) = chapter_fixtures();
		let kept = arrange_chapters(chapters, &groups, &[], &[], Some(keep));
		assert_eq!(
			kept.into_iter()
				.map(|chapter| chapter.id)
				.collect::<Vec<_>>(),
			expected
		);
	}
	let (chapters, groups) = chapter_fixtures();
	let all = arrange_chapters(chapters, &groups, &[], &[], None);
	assert_eq!(
		all.into_iter()
			.map(|chapter| chapter.id)
			.collect::<Vec<_>>(),
		[
			"a-2a",
			"a-2b",
			"b-2",
			"a-1",
			"b-1",
			"a-special",
			"b-special"
		]
	);
	let (chapters, groups) = chapter_fixtures();
	let kept = arrange_chapters(
		chapters,
		&groups,
		&["a".into()],
		&[],
		Some(settings::Keep::HighestRated),
	);
	assert_eq!(
		kept.into_iter()
			.map(|chapter| chapter.id)
			.collect::<Vec<_>>(),
		["b-2", "b-1", "b-special"]
	);
}

#[aidoku_test]
fn review_deduplication_ties() {
	for reverse in [false, true] {
		for keep in [
			settings::Keep::HighestRated,
			settings::Keep::Newest,
			settings::Keep::MostPages,
		] {
			let (mut chapters, mut groups): (Vec<models::ChapterItem>, Vec<models::Scanlator>) = (
				serde_json::from_str(
					r#"[
					{"id":"z","scanlationMangaId":"a","number":1,"createdAt":10,"pageCount":10},
					{"id":"a","scanlationMangaId":"b","number":1,"createdAt":10,"pageCount":10},
					{"id":"b","scanlationMangaId":"b","number":1,"createdAt":10,"pageCount":10}
				]"#,
				)
				.unwrap(),
				serde_json::from_str(
					r#"[{"id":"a","name":"A","score":5},{"id":"b","name":"B","score":5}]"#,
				)
				.unwrap(),
			);
			if reverse {
				chapters.reverse();
				groups.reverse();
			}
			let kept = arrange_chapters(chapters, &groups, &[], &[], Some(keep));
			assert_eq!(
				kept.into_iter()
					.map(|chapter| chapter.id)
					.collect::<Vec<_>>(),
				["a", "b"]
			);
		}
	}
}

#[aidoku_test]
fn review_group_preference() {
	for keep in [
		settings::Keep::HighestRated,
		settings::Keep::Newest,
		settings::Keep::MostPages,
	] {
		let (mut chapters, groups) = chapter_fixtures();
		chapters.push(
			serde_json::from_str(
				r#"{"id":"b-3","scanlationMangaId":"b","number":3,"createdAt":1,"pageCount":1}"#,
			)
			.unwrap(),
		);
		let kept = arrange_chapters(
			chapters,
			&groups,
			&[],
			&["a".into(), "b".into()],
			Some(keep),
		);
		assert_eq!(
			kept.into_iter()
				.filter(|chapter| chapter.number.is_some())
				.map(|chapter| chapter.id)
				.collect::<Vec<_>>(),
			["b-3", "a-2b", "a-2a", "a-1"]
		);
	}
	for preferred in [
		vec!["b".into(), "a".into()],
		vec!["missing".into(), "b".into()],
	] {
		let (chapters, groups) = chapter_fixtures();
		let kept = arrange_chapters(
			chapters,
			&groups,
			&[],
			&preferred,
			Some(settings::Keep::HighestRated),
		);
		assert_eq!(
			kept.into_iter()
				.filter(|chapter| chapter.number.is_some())
				.map(|chapter| chapter.id)
				.collect::<Vec<_>>(),
			["b-2", "b-1"]
		);
	}
	let (chapters, groups) = chapter_fixtures();
	let fallback = arrange_chapters(
		chapters,
		&groups,
		&[],
		&["missing".into()],
		Some(settings::Keep::Newest),
	);
	assert_eq!(
		fallback
			.into_iter()
			.filter(|chapter| chapter.number.is_some())
			.map(|chapter| chapter.id)
			.collect::<Vec<_>>(),
		["b-2", "a-1"]
	);
	let (chapters, groups) = chapter_fixtures();
	let excluded = arrange_chapters(
		chapters,
		&groups,
		&["a".into()],
		&["a".into(), "b".into()],
		Some(settings::Keep::HighestRated),
	);
	assert_eq!(
		excluded
			.into_iter()
			.map(|chapter| chapter.id)
			.collect::<Vec<_>>(),
		["b-2", "b-1", "b-special"]
	);
	let (chapters, groups) = chapter_fixtures();
	let all = arrange_chapters(chapters, &groups, &[], &["b".into()], None);
	assert_eq!(
		all.into_iter()
			.map(|chapter| chapter.id)
			.collect::<Vec<_>>(),
		[
			"a-2a",
			"a-2b",
			"b-2",
			"a-1",
			"b-1",
			"a-special",
			"b-special"
		]
	);
}

#[aidoku_test]
fn review_group_preference_settings() {
	assert!(settings::preferred_scanlators().is_empty());
	defaults_set("preferredScanlators", strings(&[" B ", "A, Unknown", " "]));
	assert_eq!(settings::preferred_scanlators(), ["b", "a", "unknown"]);
	defaults_set("preferredScanlators", strings(&[]));
	assert!(settings::preferred_scanlators().is_empty());
	clear(&["preferredScanlators"]);
}

#[aidoku_test]
fn chapter_settings() {
	let chapters = || {
		Atsumaru::new()
			.get_manga_update(manga(COMIC), false, true)
			.unwrap()
			.chapters
			.unwrap()
	};
	let unique = |list: &[Chapter]| {
		let mut numbers = list
			.iter()
			.filter_map(|chapter| chapter.chapter_number.map(f32::to_bits))
			.collect::<Vec<_>>();
		let total = numbers.len();
		numbers.sort_unstable();
		numbers.dedup();
		numbers.len() == total
	};
	let all = chapters();
	assert!(!unique(&all));

	defaults_set("deduplicateChapters", DefaultValue::Bool(true));
	for keep in ["rated", "newest", "pages"] {
		defaults_set("deduplicateKeep", DefaultValue::String(keep.into()));
		let list = chapters();
		assert!(list.len() > 100 && list.len() <= all.len(), "{keep}");
		for chapter in &list {
			if let Some(number) = chapter.chapter_number {
				assert!(
					list.iter()
						.filter(|other| other.chapter_number == Some(number))
						.all(|other| other.scanlators == chapter.scanlators),
					"{keep}: {number}"
				);
			}
		}
	}
	clear(&["deduplicateChapters", "deduplicateKeep"]);

	let group = all[0].scanlators.as_ref().unwrap()[0].clone();
	defaults_set(
		"excludedScanlators",
		strings(&[&format!("{}, nobody", group.to_uppercase())]),
	);
	let filtered = chapters();
	clear(&["excludedScanlators"]);
	assert!(filtered.len() < all.len());
	assert!(filtered.iter().all(|chapter| {
		chapter
			.scanlators
			.as_ref()
			.is_none_or(|names| names[0] != group)
	}));
}

#[aidoku_test]
fn novel() {
	let source = Atsumaru::new();
	let novel = source.get_manga_update(manga(NOVEL), true, true).unwrap();
	assert_eq!(novel.url.as_deref(), Some("https://atsu.moe/novel/slh2"));
	assert_eq!(novel.key, NOVEL);
	let chapter = novel.chapters.as_ref().unwrap().last().unwrap().clone();
	assert!(
		chapter
			.url
			.as_deref()
			.unwrap()
			.starts_with("https://atsu.moe/novel/slh2/")
	);
	assert!(
		novel
			.chapters
			.as_ref()
			.unwrap()
			.iter()
			.all(|chapter| !chapter.key.starts_with(NOVEL_PREFIX))
	);
	let pages = source
		.get_page_list(
			manga(NOVEL),
			Chapter {
				key: chapter.key,
				..Default::default()
			},
		)
		.unwrap();
	assert_eq!(pages.len(), 1);
	assert!(matches!(&pages[0].content, PageContent::Text(text) if text.len() > 1000));
	let chapters_only = source.get_manga_update(manga(NOVEL), false, true).unwrap();
	assert_eq!(chapters_only.key, NOVEL);
	assert!(
		chapters_only
			.chapters
			.unwrap()
			.iter()
			.all(|chapter| !chapter.key.starts_with(NOVEL_PREFIX))
	);

	let labelled = source.get_manga_update(manga(NOVEL), true, false).unwrap();
	defaults_set("labelNovels", DefaultValue::Bool(false));
	let plain = source.get_manga_update(manga(NOVEL), true, false).unwrap();
	clear(&["labelNovels"]);
	assert!(labelled.title.ends_with(" (Novel)"));
	assert!(!plain.title.ends_with(" (Novel)"));
}

#[aidoku_test]
fn unknown_series() {
	let error = Atsumaru::new()
		.get_manga_update(manga("zzzzzz"), true, true)
		.unwrap_err();
	assert!(format!("{error:?}").contains("no longer on Atsumaru"));
}

#[aidoku_test]
fn deep_links() {
	let source = Atsumaru::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert!(
		matches!(link("https://atsu.moe/manga/n86El"), Some(DeepLinkResult::Manga { key }) if key == COMIC)
	);
	assert!(
		matches!(link("https://atsu.moe/novel/slh2"), Some(DeepLinkResult::Manga { key }) if key == NOVEL)
	);
	assert!(matches!(
		link("https://atsu.moe/read/n86El/3uheUh"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == COMIC && key == "3uheUh"
	));
	assert!(matches!(
		link("https://atsu.moe/novel/slh2/5nDftC?p=1"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == NOVEL && key == "5nDftC"
	));
	assert!(link("https://atsu.moe/explore").is_none());
	assert!(link("https://atsu.moe/read/n86El").is_none());
}

#[aidoku_test]
fn chapter_titles() {
	use helpers::chapter_title;
	assert_eq!(chapter_title("Chapter 143", Some(143.0)), None);
	assert_eq!(
		chapter_title("Chapter 1 - Crimson", Some(1.0)).as_deref(),
		Some("Crimson")
	);
	assert_eq!(
		chapter_title("Chapters of Fate", Some(1.0)).as_deref(),
		Some("Chapters of Fate")
	);
	assert_eq!(
		chapter_title("Chapter 5", None).as_deref(),
		Some("Chapter 5")
	);
}
