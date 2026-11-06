#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	ContentRating, MangaStatus,
	imports::defaults::{DefaultValue, defaults_set},
};
use aidoku_test::aidoku_test;
use helpers::{ADULT_GENRES, chapter_title, content_rating, genres};

// An adult novel with 53 named chapters, so it only shows with adult titles on.
const NOVEL_KEY: &str = "6aedf2f22fe18aff37a5a186";

const LIST: &str = r#"{"novels":[{"id":"6aedf3372fe18aff37a5a1c8","author":"MrBizzarre","chapter_names":[],"genres":"Fantasy,Romance,System,Slice of Life","novel_image":"/api/novels/6aedf3372fe18aff37a5a1c8/cover?w=640&q=72&format=webp","image_url":"/api/novels/6aedf3372fe18aff37a5a1c8/cover?w=640&q=72&format=webp","cover_url":"/api/novels/6aedf3372fe18aff37a5a1c8/cover?w=640&q=72&format=webp","title":"My Family Man System: Taking Care of My Family Makes Me OP","description":"Jensen Atlas was born *into* one of the most influential Hunter families.","total_chapters":"58","rating":0,"release_status":"ongoing","updated_at":"2026-11-06T15:33:11.410Z"}],"pagination":{"page":1,"per_page":1,"total":272277,"total_pages":272277,"has_prev":false,"has_next":true}}"#;

const DETAILS: &str = r#"{"novel":{"id":"6aedf2f22fe18aff37a5a186","author":"DemonGodWrites","chapter_names":["Chapter 1: The Blade at His Throat","Chapter 2: The Art They Called Weak","Chapter 3"],"genres":"Eastern,Adult,System,Harem,Smut","cover_url":"/api/novels/6aedf2f22fe18aff37a5a186/cover?w=640&q=72&format=webp","title":"My Dual Cultivation Harem in the Demonic Cult","description":"The most scandalous murim novel on the internet.","total_chapters":"3","release_status":"ongoing"}}"#;

fn novel(json: &str) -> Novel {
	serde_json::from_str::<NovelDetails>(json).unwrap().novel
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	NovelArchive::new()
		.get_search_manga_list(query.map(Into::into), 1, filters)
		.unwrap()
}

fn sort(index: i32) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending: false,
	}
}

fn genre_filter(included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: "genre".into(),
		included: included.iter().map(|genre| (*genre).into()).collect(),
		excluded: excluded.iter().map(|genre| (*genre).into()).collect(),
	}
}

fn manga(key: &str) -> Manga {
	Manga {
		key: key.into(),
		..Default::default()
	}
}

#[aidoku_test]
fn review_search_url_sends_what_the_site_sends() {
	let plain = SearchParams::default().url(2, false);
	assert_eq!(plain, format!("{API_URL}?page=2&per_page=24"));

	let filtered = SearchParams {
		query: Some("demon cult".into()),
		sort: Some("rating"),
		status: Some("completed".into()),
		any_genre: true,
		genres: vec!["fantasy".into(), "harem".into()],
		hidden_genres: vec!["romance".into()],
	}
	.url(1, false);
	for part in [
		"search=demon%20cult",
		"fuzzy=1",
		"sort=rating",
		"status=completed",
		"genres_include=fantasy%2Charem",
		"genres_exclude=romance",
		"genre_match=any",
	] {
		assert!(filtered.contains(part), "{filtered} lacks {part}");
	}
}

#[aidoku_test]
fn review_hiding_adult_titles_adds_the_site_exclusions() {
	let url = SearchParams {
		hidden_genres: vec!["romance".into()],
		..Default::default()
	}
	.url(1, true);
	let excluded = url.split("genres_exclude=").nth(1).unwrap();
	assert!(excluded.starts_with("romance%2Cadult%2Csmut"));
	assert_eq!(excluded.matches("%2C").count(), ADULT_GENRES.len());
}

#[aidoku_test]
fn review_chapter_titles_drop_only_their_own_number() {
	assert_eq!(
		chapter_title("Chapter 1: The Blade at His Throat", 1).as_deref(),
		Some("The Blade at His Throat")
	);
	assert_eq!(
		chapter_title("Chapter 174 I Am The Only God!", 174).as_deref(),
		Some("I Am The Only God!")
	);
	assert_eq!(chapter_title("Chapter 3", 3), None);
	assert_eq!(
		chapter_title("Chapter 10", 1).as_deref(),
		Some("Chapter 10")
	);
	assert_eq!(chapter_title("Prologue", 1).as_deref(), Some("Prologue"));
}

#[aidoku_test]
fn review_genres_drop_duplicates_and_categories() {
	assert_eq!(
		genres("Fantasy, fantasy,Browse,,Romance"),
		vec![String::from("Fantasy"), String::from("Romance")]
	);
}

#[aidoku_test]
fn review_content_rating_follows_the_genres() {
	assert_eq!(content_rating("Eastern,Adult,Smut"), ContentRating::NSFW);
	assert_eq!(content_rating("Ecchi,Smut"), ContentRating::NSFW);
	assert_eq!(content_rating("Fantasy,Ecchi"), ContentRating::Suggestive);
	assert_eq!(content_rating("Fantasy,Romance"), ContentRating::Safe);
}

#[aidoku_test]
fn review_cards_carry_only_key_title_cover_and_rating() {
	let list: NovelList = serde_json::from_str(LIST).unwrap();
	assert!(list.pagination.unwrap().has_next);
	let card = Manga::from(list.novels.into_iter().next().unwrap());
	assert_eq!(card.key, "6aedf3372fe18aff37a5a1c8");
	assert_eq!(
		card.cover.as_deref(),
		Some(
			"https://novelarchive.st/api/novels/6aedf3372fe18aff37a5a1c8/cover?w=640&q=72&format=webp"
		)
	);
	assert!(card.description.is_none() && card.tags.is_none() && card.authors.is_none());
}

#[aidoku_test]
fn review_details_read_the_captured_novel() {
	let details = novel(DETAILS).into_details();
	assert_eq!(details.key, NOVEL_KEY);
	assert_eq!(
		details.title,
		"My Dual Cultivation Harem in the Demonic Cult"
	);
	assert_eq!(details.authors, Some(vec![String::from("DemonGodWrites")]));
	assert_eq!(details.status, MangaStatus::Ongoing);
	assert_eq!(details.content_rating, ContentRating::NSFW);
	assert_eq!(details.tags.unwrap().len(), 5);
	assert_eq!(
		details.url.as_deref(),
		Some("https://novelarchive.st/novel?id=6aedf2f22fe18aff37a5a186")
	);
	// Descriptions are Markdown, so the site's punctuation is escaped.
	assert!(details.description.unwrap().ends_with("internet\\."));
}

#[aidoku_test]
fn review_chapters_are_positions_newest_first() {
	let chapters = novel(DETAILS).chapters();
	let keys: Vec<&str> = chapters
		.iter()
		.map(|chapter| chapter.key.as_str())
		.collect();
	assert_eq!(keys, ["3", "2", "1"]);
	assert_eq!(
		chapters[2].title.as_deref(),
		Some("The Blade at His Throat")
	);
	assert_eq!(chapters[0].title, None);
	assert_eq!(
		chapters[0].url.as_deref(),
		Some("https://novelarchive.st/reader?novel=6aedf2f22fe18aff37a5a186&chapter=3")
	);
}

#[aidoku_test]
fn review_chapters_fall_back_to_the_count() {
	let chapters = novel(r#"{"novel":{"id":"a","title":"A","total_chapters":"4"}}"#).chapters();
	assert_eq!(chapters.len(), 4);
	assert_eq!(chapters[0].chapter_number, Some(4.0));
	assert!(chapters.iter().all(|chapter| chapter.title.is_none()));
}

#[aidoku_test]
fn review_latest_rows_name_the_counted_chapter() {
	let list: NovelList = serde_json::from_str(LIST).unwrap();
	let latest = list
		.novels
		.into_iter()
		.next()
		.unwrap()
		.into_latest()
		.unwrap();
	assert_eq!(latest.chapter.key, "58");
	// The runner's date parser takes no quoted literals, so the date is only checked in the app.
	assert!(latest.manga.description.is_none());
	assert!(
		novel(r#"{"novel":{"id":"a","title":"A"}}"#)
			.into_latest()
			.is_none()
	);
}

#[aidoku_test]
fn review_deep_links_match_the_site_urls() {
	let source = NovelArchive::new();
	let link = |url: &str| source.handle_deep_link(url.into()).unwrap();
	assert!(matches!(
		link("https://novelarchive.st/novel?id=6aedf2f22fe18aff37a5a186"),
		Some(DeepLinkResult::Manga { key }) if key == NOVEL_KEY
	));
	assert!(matches!(
		link("https://novelarchive.st/reader?novel=6aedf2f22fe18aff37a5a186&chapter=12#top"),
		Some(DeepLinkResult::Chapter { manga_key, key }) if manga_key == NOVEL_KEY && key == "12"
	));
	assert!(link("https://novelarchive.st/library").is_none());
	assert!(link("https://novelarchive.st/novel?id=").is_none());
	assert!(link("https://example.com/novel?id=1").is_none());
}

#[aidoku_test]
fn search_pages_and_sorts() {
	let first = search(None, vec![]);
	let second = NovelArchive::new()
		.get_search_manga_list(None, 2, vec![])
		.unwrap();
	assert_eq!(first.entries.len(), PAGE_SIZE as usize);
	assert!(first.has_next_page);
	assert_ne!(first.entries[0].key, second.entries[0].key);
	assert_ne!(
		search(None, vec![sort(1)]).entries[0].key,
		search(None, vec![sort(3)]).entries[0].key
	);
}

#[aidoku_test]
fn search_finds_a_title() {
	let results = search(Some("Dual Cultivation Harem in the Demonic Cult"), vec![]);
	// Adult titles are hidden by default, as on the site.
	assert!(!results.entries.iter().any(|manga| manga.key == NOVEL_KEY));
	defaults_set("hideAdult", DefaultValue::Bool(false));
	let results = search(Some("Dual Cultivation Harem in the Demonic Cult"), vec![]);
	assert!(results.entries.iter().any(|manga| manga.key == NOVEL_KEY));
}

#[aidoku_test]
fn genre_filter_includes_and_excludes() {
	let source = NovelArchive::new();
	let tags = |filter: FilterValue| {
		let first = search(None, vec![filter]).entries.remove(0);
		source
			.get_manga_update(first, true, false)
			.unwrap()
			.tags
			.unwrap_or_default()
			.iter()
			.map(|tag| tag.to_lowercase())
			.collect::<Vec<_>>()
	};
	assert!(tags(genre_filter(&["harem"], &[])).contains(&String::from("harem")));
	assert!(!tags(genre_filter(&[], &["fantasy"])).contains(&String::from("fantasy")));
}

#[aidoku_test]
fn hidden_adult_titles_stay_out_of_search() {
	let results = search(None, vec![]);
	assert!(
		results
			.entries
			.iter()
			.all(|manga| manga.content_rating != ContentRating::NSFW)
	);
}

#[aidoku_test]
fn details_and_chapters_load() {
	let manga = NovelArchive::new()
		.get_manga_update(manga(NOVEL_KEY), true, true)
		.unwrap();
	assert_eq!(manga.title, "My Dual Cultivation Harem in the Demonic Cult");
	let chapters = manga.chapters.unwrap();
	assert!(chapters.len() >= 53);
	assert_eq!(chapters.last().unwrap().key, "1");
	assert_eq!(
		chapters.last().unwrap().title.as_deref(),
		Some("The Blade at His Throat")
	);
}

#[aidoku_test]
fn chapter_text_loads() {
	let source = NovelArchive::new();
	let chapter = Chapter {
		key: "1".into(),
		..Default::default()
	};
	let pages = source.get_page_list(manga(NOVEL_KEY), chapter).unwrap();
	let PageContent::Text(text) = &pages[0].content else {
		unreachable!("a novel chapter is text");
	};
	assert!(text.contains("Kang Do\\-yun"));
}

#[aidoku_test]
fn unknown_novel_fails_with_the_site_message() {
	let error = NovelArchive::new()
		.get_manga_update(manga("000000000000000000000000"), true, false)
		.err()
		.unwrap();
	assert!(format!("{error:?}").contains("Novel not found"));
}

#[aidoku_test]
fn home_has_the_site_sections() {
	let titles: Vec<String> = NovelArchive::new()
		.get_home()
		.unwrap()
		.components
		.into_iter()
		.filter_map(|component| component.title)
		.collect();
	assert_eq!(
		titles,
		[
			"Trending",
			"Editor's Choice",
			"Popular",
			"Recently Updated",
			"Top Rated",
			"New in the Archive"
		]
	);
}

#[aidoku_test]
fn home_listings_page() {
	let source = NovelArchive::new();
	for id in ["popular", "rating", "recent"] {
		let listing = Listing {
			id: id.into(),
			..Default::default()
		};
		let first = source.get_manga_list(listing.clone(), 1).unwrap();
		let second = source.get_manga_list(listing, 2).unwrap();
		assert!(first.has_next_page);
		assert_ne!(first.entries[0].key, second.entries[0].key);
	}
	assert!(
		source
			.get_manga_list(
				Listing {
					id: "unknown".into(),
					..Default::default()
				},
				1
			)
			.is_err()
	);
}

#[aidoku_test]
fn genre_filter_lists_the_site_genres() {
	let filters = NovelArchive::new().get_dynamic_filters().unwrap();
	assert_eq!(filters.len(), 1);
}
