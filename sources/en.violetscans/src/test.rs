#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, LinkValue, Manga,
	MangaPageResult, MangaStatus, PageContent,
	alloc::{String, Vec, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

// The site's genre id for Romance, from res/filters.json.
const GENRE: &str = "8";

// The home cut down to one card per row, in the site's markup and order.
const HOME: &str = r#"
<div class="slidernew"><div class="swiper-slide"><a href="https://violetmanga.com/comics/banner/"><img src="https://img.test/banner.jpg" alt="Banner"></a></div></div>
<div class="hotslid"><div class="bixbox"><div class="releases"><h2>Popular Today</h2></div></div>
<div class="rail"><div class="bsx"><a href="https://violetmanga.com/comics/hot/" title="Hot"><img src="https://img.test/hot.jpg"></a></div></div></div>
<div class="bixbox"><div class="releases"><h2>Genres</h2></div><a href="/genres/romance/">Romance</a></div>
<div class="bixbox"><div class="releases"><h2>Latest Comics</h2></div><div id="manga-posts">
<div class="bsx"><a href="https://violetmanga.com/comics/new/" title="New"><img src="https://img.test/new.jpg"></a>
<div class="chapter-list"><a href="https://violetmanga.com/new-chapter-2/"><span class="epxs">Chapter 2</span><i class="fa-coins"></i></a>
<a href="https://violetmanga.com/new-chapter-1/"><span class="epxs">Chapter 1</span></a></div></div></div></div>
<div class="bixbox"><div class="releases"><h2>Latest Novels</h2></div>
<div class="bsx"><a href="https://violetmanga.com/comics/novel/" title="Novel"><img src="https://img.test/novel.jpg"></a>
<div class="chapter-list"><a href="https://violetmanga.com/novel-chapter-9/"><span class="epxs">Chapter 9</span><i class="fa-coins"></i></a></div></div></div>
<section class="bixbox"><div class="releases"><h2>Editor Picks</h2></div>
<div class="violet-ep-cover" data-index="0"><img src="https://img.test/pick.jpg"></div>
<div class="violet-ep-panel" data-index="0"><div class="violet-ep-series-title"><a href="https://violetmanga.com/comics/pick/">Pick</a></div>
<span class="violet-ep-badge--status">Ongoing</span><p class="violet-ep-tagline">A plain tagline.</p><ul class="violet-ep-genres"><li>Romance</li></ul></div></section>
"#;

fn source() -> MangaThemesia<VioletScans> {
	MangaThemesia::new()
}

fn search(query: Option<&str>, page: i32, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(String::from), page, filters)
		.unwrap()
}

fn keys(result: &MangaPageResult) -> Vec<&str> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.as_str())
		.collect()
}

fn first_series() -> Manga {
	search(None, 1, vec![]).entries.remove(0)
}

#[aidoku_test]
fn review_home_keeps_the_owner_layout() {
	let source = VioletScans::new();
	let layout = child_theme::parse_home(&source.params(), &Html::parse(HOME).unwrap());
	let rows: Vec<Option<&str>> = layout
		.components
		.iter()
		.map(|component| component.title.as_deref())
		.collect();
	// Genre tiles hold no series, so they make no row
	assert_eq!(
		rows,
		[
			None,
			Some("Popular Today"),
			Some("Latest Comics"),
			Some("Latest Novels"),
			Some("Editor Picks")
		]
	);
	let values: Vec<&HomeComponentValue> = layout
		.components
		.iter()
		.map(|component| &component.value)
		.collect();
	let HomeComponentValue::ImageScroller {
		links,
		width,
		height,
		..
	} = values[0]
	else {
		panic!("the banner rail comes first");
	};
	assert_eq!((*width, *height), (Some(400), Some(160)));
	assert!(
		matches!(&links[0].value, Some(LinkValue::Manga(manga)) if manga.key == "/comics/banner/")
	);
	assert!(
		matches!(values[1], HomeComponentValue::Scroller { entries, .. } if entries[0].title == "Hot" && entries[0].subtitle.is_none())
	);
	// Latest Comics names the newest chapter a reader can open, skipping the coin one
	let HomeComponentValue::MangaChapterList { entries, .. } = values[2] else {
		panic!("latest comics is a chapter list");
	};
	assert_eq!(entries[0].chapter.key, "/new-chapter-1/");
	assert_eq!(entries[0].chapter.chapter_number, Some(1.0));
	// Latest Novels stays cards, since its newest chapters are mostly coin-locked
	assert!(
		matches!(values[3], HomeComponentValue::Scroller { entries, .. } if entries[0].title == "Novel")
	);
	let HomeComponentValue::BigScroller { entries, .. } = values[4] else {
		panic!("editor picks are a big scroller");
	};
	let pick = &entries[0];
	assert_eq!(pick.key, "/comics/pick/");
	assert_eq!(pick.cover.as_deref(), Some("https://img.test/pick.jpg"));
	assert_eq!(pick.description.as_deref(), Some("A plain tagline."));
	assert_eq!(pick.status, MangaStatus::Ongoing);
}

#[aidoku_test]
fn browse_pages_through_the_library() {
	let first = search(None, 1, vec![]);
	let second = search(None, 2, vec![]);
	assert!(first.has_next_page);
	assert!(!first.entries.is_empty() && !second.entries.is_empty());
	assert_ne!(keys(&first), keys(&second));
	// Cards carry only what search lists.
	assert!(
		first
			.entries
			.iter()
			.all(|manga| manga.description.is_none())
	);
}

#[aidoku_test]
fn novels_read_as_text() {
	let novel = search(
		None,
		1,
		vec![FilterValue::Select {
			id: "type".into(),
			value: "novel".into(),
		}],
	)
	.entries
	.remove(0);
	let manga = source().get_manga_update(novel, false, true).unwrap();
	let chapters = manga.chapters.clone().unwrap();
	for chapter in [chapters.first(), chapters.last()].into_iter().flatten() {
		let pages = source()
			.get_page_list(manga.clone(), chapter.clone())
			.unwrap();
		let [page] = pages.as_slice() else {
			panic!("one text page, got {}", pages.len());
		};
		let PageContent::Text(text) = &page.content else {
			panic!("a novel chapter is text");
		};
		assert!(text.len() > 1000);
		assert!(!text.contains("<p") && !text.contains("<div"));
	}
}

#[aidoku_test]
fn search_finds_a_title() {
	let series = first_series();
	// Letters and digits only: the site's search doesn't match curly quotes or ellipses.
	let word = series
		.title
		.split(|c: char| !c.is_alphanumeric())
		.max_by_key(|word| word.len())
		.unwrap();
	let found = search(Some(word), 1, vec![]);
	assert!(keys(&found).contains(&series.key.as_str()), "{word}");
	assert!(search(Some("zzqxwvnomatch"), 1, vec![]).entries.is_empty());
}

#[aidoku_test]
fn sorts_follow_the_site() {
	let titles = |index| {
		search(
			None,
			1,
			vec![FilterValue::Sort {
				id: "order".into(),
				index,
				ascending: false,
			}],
		)
		.entries
		.into_iter()
		.map(|manga| manga.title.to_lowercase())
		.collect::<Vec<_>>()
	};
	let a_to_z = titles(1);
	let z_to_a = titles(2);
	assert!(a_to_z.windows(2).filter(|pair| pair[0] <= pair[1]).count() * 10 >= a_to_z.len() * 8);
	assert_ne!(a_to_z, z_to_a);
	assert_ne!(titles(5), titles(4));
}

#[aidoku_test]
fn status_type_and_genre_filters_narrow_results() {
	let select = |id: &str, value: &str| FilterValue::Select {
		id: id.into(),
		value: value.into(),
	};
	// The runner can't parse the template's `:contains` selectors, so details have no status or
	// tags here; each filter is checked by what it changes in the results instead.
	let all = keys(&search(None, 1, vec![]))
		.into_iter()
		.map(String::from)
		.collect::<Vec<_>>();
	let narrowed = |filter| {
		keys(&search(None, 1, vec![filter]))
			.into_iter()
			.map(String::from)
			.collect::<Vec<_>>()
	};
	let ongoing = narrowed(select("status", "ongoing"));
	assert!(!ongoing.is_empty());
	assert_ne!(ongoing, narrowed(select("status", "completed")));
	assert_ne!(
		narrowed(select("type", "manhwa")),
		narrowed(select("type", "manga"))
	);
	let genre = narrowed(FilterValue::MultiSelect {
		id: "genre[]".into(),
		included: vec![GENRE.into()],
		excluded: vec![],
	});
	assert!(!genre.is_empty());
	assert_ne!(genre, all);
}

#[aidoku_test]
fn author_and_year_filters_narrow_results() {
	let text = |id: &str, value: &str| FilterValue::Text {
		id: id.into(),
		value: value.into(),
	};
	assert!(!search(None, 1, vec![]).entries.is_empty());
	assert!(
		search(None, 1, vec![text("author", "zzqxwvnobody")])
			.entries
			.is_empty()
	);
	assert!(
		search(None, 1, vec![text("yearx", "1901")])
			.entries
			.is_empty()
	);
}

#[aidoku_test]
fn details_chapters_and_pages_load() {
	let manga = source()
		.get_manga_update(first_series(), true, true)
		.unwrap();
	assert!(!manga.title.is_empty() && manga.cover.is_some());
	assert!(
		manga
			.url
			.as_deref()
			.is_some_and(|url| url.starts_with(BASE_URL))
	);
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty());
	assert!(chapters.iter().all(|chapter| chapter.url.is_some()));
	let pages = source()
		.get_page_list(manga, chapters.last().unwrap().clone())
		.unwrap();
	assert!(!pages.is_empty());
}

#[aidoku_test]
fn deep_links_open_series() {
	let manga = source()
		.get_manga_update(first_series(), true, false)
		.unwrap();
	assert_eq!(
		source().handle_deep_link(manga.url.unwrap()).unwrap(),
		Some(DeepLinkResult::Manga { key: manga.key })
	);
	// The template opens series links only.
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}/zzqxwv-no-such-page/"))
			.unwrap(),
		None
	);
}

#[aidoku_test]
fn an_unknown_series_fails() {
	let missing = Manga {
		key: "/comics/zzqxwv-no-such-series/".into(),
		..Default::default()
	};
	// The site's 404 page either fails to parse or lists no chapters.
	let result = source().get_manga_update(missing, true, true);
	assert!(result.map_or(true, |manga| manga.chapters.unwrap_or_default().is_empty()));
}

#[aidoku_test]
fn home_follows_the_site() {
	let home = source().get_home().unwrap();
	let HomeComponentValue::ImageScroller { links, .. } = &home.components[0].value else {
		panic!("the banner rail comes first");
	};
	assert!(links.len() >= 3);
	let mut keys = links
		.iter()
		.filter_map(|link| match &link.value {
			Some(LinkValue::Manga(manga)) => Some(manga.key.as_str()),
			_ => None,
		})
		.collect::<Vec<_>>();
	assert_eq!(keys.len(), links.len());
	keys.dedup();
	assert_eq!(keys.len(), links.len(), "banners repeat");
	// Genre tiles hold no series, so they make no row.
	let titles = home.components[1..]
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect::<Vec<_>>();
	assert_eq!(
		titles,
		[
			"Popular Today",
			"New Series",
			"Latest Manga",
			"Latest Comics",
			"Latest Novels",
			"Editor Picks",
			"Completed Series"
		]
	);
	let picks = home
		.components
		.iter()
		.find(|component| component.title.as_deref() == Some("Editor Picks"))
		.unwrap();
	let HomeComponentValue::BigScroller { entries, .. } = &picks.value else {
		panic!("editor picks are a big scroller");
	};
	assert!(entries.iter().all(|manga| manga.cover.is_some()
		&& manga.description.is_some()
		&& manga.status != MangaStatus::Unknown));
}

// Latest Comics names each series' newest chapter a reader can open, keyed as its chapter list is.
#[aidoku_test]
fn latest_comics_name_readable_chapters() {
	let home = source().get_home().unwrap();
	let latest = home
		.components
		.iter()
		.find(|component| component.title.as_deref() == Some("Latest Comics"))
		.unwrap();
	let HomeComponentValue::MangaChapterList { entries, .. } = &latest.value else {
		panic!("latest comics is a chapter list");
	};
	assert!(!entries.is_empty());
	assert!(
		entries
			.iter()
			.all(|entry| entry.chapter.chapter_number.is_some())
	);
	let entry = &entries[0];
	let manga = source()
		.get_manga_update(entry.manga.clone(), false, true)
		.unwrap();
	assert!(
		manga
			.chapters
			.clone()
			.unwrap()
			.iter()
			.any(|chapter| chapter.key == entry.chapter.key)
	);
	let pages = source()
		.get_page_list(manga, entry.chapter.clone())
		.unwrap();
	assert!(!pages.is_empty());
}

// Latest Novels stays a row of cards: its newest chapters are mostly coin-locked.
#[aidoku_test]
fn latest_novels_are_cards() {
	let home = source().get_home().unwrap();
	let novels = home
		.components
		.iter()
		.find(|component| component.title.as_deref() == Some("Latest Novels"))
		.unwrap();
	let HomeComponentValue::Scroller { entries, .. } = &novels.value else {
		panic!("latest novels is a scroller");
	};
	assert!(!entries.is_empty());
}
