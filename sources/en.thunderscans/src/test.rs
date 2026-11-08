#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, LinkValue, Manga,
	MangaPageResult, PageContent,
	alloc::{String, Vec, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

// The site's genre id for Romance, from res/filters.json.
const GENRE: &str = "128";

fn source() -> MangaThemesia<ThunderScans> {
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

// The site puts two lines of dialogue in one paragraph, split by a line break.
#[aidoku_test]
fn review_novel_lines_split_at_line_breaks() {
	let html = Html::parse(
		"<div id=\"readerarea\"><p>“Good work.”<br/>\n“Yes. I have <i>taken</i> care of it.”</p>\
		<p>1. Then &amp; now</p><div><p>*Nested*</p></div></div>",
	)
	.unwrap();
	assert_eq!(
		mangathemesia::helpers::novel_text(&html),
		"“Good work.”\n\n“Yes. I have taken care of it.”\n\n1\\. Then \\& now\n\n\\*Nested\\*"
	);
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
	let titles = home.components[1..]
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect::<Vec<_>>();
	assert_eq!(
		titles,
		[
			"Popular Today",
			"Editor's Pick",
			"Latest Comics",
			"Latest Novels"
		]
	);
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
