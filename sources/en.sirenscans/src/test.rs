#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, HomeLayout,
	Manga, MangaPageResult, MangaStatus, PageContent, Viewer,
	alloc::{String, Vec, format, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

// A genre, a type and a status from res/filters.json that each narrow the catalogue
const GENRE: &str = "action";
const GENRE_NAME: &str = "Action";
const TYPE: &str = "manhwa";
const STATUS: &str = "completed";

// The site's home rows as it renders them, cut down to a card or two each
const HOME: &str = r#"
<section class="series-slider-section"><h2>Featured Series</h2>
	<a href="{base}/first-series" data-slide-card class="slide-img-card group">
		<img alt="First Series" data-src="https://cdn.test/first.avif"><h3> First Series </h3></a>
	<a href="/novel-one" data-slide-card class="slider4-card" title="[ Novel ] Novel One">
		<img data-src="https://cdn.test/novel.avif"></a>
</section>
<section class="poster-slider-section"><h2>Novels</h2><div id="novel-slider">
	<a href="/novel-one" data-slide-card title="[ Novel ] Novel One"><img data-src="https://cdn.test/novel.avif"></a>
	<a href="/untitled-novel" data-slide-card title="Untitled Novel"><img data-src="https://cdn.test/novel.avif"></a>
</div></section>
<section id="recent-updated-section"><h2>Recently Updated</h2>
	<article class="ru-card">
		<a class="ru-cover" href="/second-series"><img data-src="https://cdn.test/second.avif"></a>
		<a class="ru-newbar" href="/second-series-chapter-12">New Chapter</a>
		<h2 class="ru-title"><a href="/second-series">Second Series</a></h2>
		<a class="ru-row ru-row--paid" href="/second-series-chapter-12"><span class="ru-ch">Chapter 12</span>
			<span class="ru-gate"><img alt="75 coins"></span><time>4h ago</time></a>
		<a class="ru-row" href="/second-series-chapter-11"><span class="ru-ch">Chapter 11</span>
			<span class="ru-new">New</span><time>5h ago</time></a>
	</article>
</section>
<section id="home-editors-choice"><h2>Editor's Choice</h2>
	<a href="/first-series" class="ec-card" data-ec-card title="First Series"><img data-src="https://cdn.test/first.avif"></a>
	<a href="/first-series" class="ec-card" data-ec-card data-ec-clone><img data-src="https://cdn.test/first.avif"></a>
</section>
<section class="poster-slider-section"><h2>Editor&apos;s Choice</h2>
	<a href="/first-series" data-slide-card title="First Series"><img data-src="https://cdn.test/first.avif"></a>
</section>
<section id="home-newly-added"><h2>Newly Added Series</h2>
	<a href="/third-series" class="group"><img data-src="https://cdn.test/third.avif"><h2>Third Series</h2></a>
	<a href="/novel-two" class="group"><img data-src="https://cdn.test/novel.avif"><h2>Novel Two</h2><span>Novel</span></a>
</section>
"#;

fn source() -> KeyoappV2<SirenScans> {
	KeyoappV2::new()
}

fn search_page(query: Option<&str>, page: i32, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(String::from), page, filters)
		.unwrap()
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	search_page(query, 1, filters)
}

fn select(id: &str, value: &str) -> FilterValue {
	FilterValue::Select {
		id: id.into(),
		value: value.into(),
	}
}

fn genres(included: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: "genre".into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: Vec::new(),
	}
}

fn sort(index: i32) -> FilterValue {
	FilterValue::Sort {
		id: "sort".into(),
		index,
		ascending: false,
	}
}

fn details(manga: Manga) -> Manga {
	source().get_manga_update(manga, true, false).unwrap()
}

// The first listed series with a chapter, since a new series can have none yet
fn with_chapters() -> (Manga, Vec<Chapter>) {
	search(None, vec![])
		.entries
		.into_iter()
		.find_map(|manga| {
			let chapters = source()
				.get_manga_update(manga.clone(), false, true)
				.unwrap()
				.chapters
				.filter(|chapters| !chapters.is_empty())?;
			Some((manga, chapters))
		})
		.unwrap()
}

fn keys(result: &MangaPageResult) -> Vec<&str> {
	result
		.entries
		.iter()
		.map(|manga| manga.key.as_str())
		.collect()
}

fn home_fixture() -> HomeLayout {
	let html = Html::parse(HOME.replace("{base}", BASE_URL)).unwrap();
	SirenScans.parse_home(&SirenScans.params(), &html)
}

#[aidoku_test]
fn review_filters_are_include_only() {
	// As on the site, genres can't be excluded
	let filters = include_str!("../res/filters.json");
	assert_eq!(filters.matches("\"canExclude\": false").count(), 1);
	assert!(!filters.contains("\"canExclude\": true"));
}

#[aidoku_test]
fn review_home_keeps_the_site_rows() {
	let home = home_fixture();
	let titles: Vec<&str> = home
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	// In the site's order, novels included, without the repeated Editor's Choice
	assert_eq!(
		titles,
		[
			"Featured Series",
			"Novels",
			"Recently Updated",
			"Editor's Choice",
			"Newly Added Series"
		]
	);
	for component in &home.components {
		match &component.value {
			HomeComponentValue::Scroller { entries, .. } => {
				assert!(!entries.is_empty());
				assert!(entries.iter().all(|link| link.subtitle.is_none()));
			}
			HomeComponentValue::MangaChapterList {
				page_size, entries, ..
			} => {
				assert_eq!(*page_size, Some(5));
				// The newest free chapter, since coin ones need a login
				assert_eq!(entries.len(), 1);
				assert_eq!(entries[0].manga.key, "/second-series");
				assert_eq!(entries[0].chapter.key, "/second-series-chapter-11");
				assert_eq!(entries[0].chapter.chapter_number, Some(11.0));
				assert!(entries[0].chapter.date_uploaded.is_some());
			}
			_ => panic!("unexpected component"),
		}
	}
	let HomeComponentValue::Scroller { entries, .. } = &home.components[0].value else {
		panic!("featured series is a scroller");
	};
	let titles: Vec<&str> = entries.iter().map(|link| link.title.as_str()).collect();
	assert_eq!(titles, ["First Series", "[ Novel ] Novel One"]);
}

#[aidoku_test]
fn search_pages_through_the_catalogue() {
	let first = search(None, vec![]);
	assert!(first.has_next_page);
	assert!(first.entries.len() > 10);
	// Cards carry only what the list shows
	assert!(first.entries.iter().all(|manga| {
		manga.key.starts_with('/') && manga.cover.is_some() && manga.description.is_none()
	}));
	let second = search_page(None, 2, vec![]);
	assert!(!second.entries.is_empty());
	assert!(keys(&second).iter().all(|key| !keys(&first).contains(key)));
}

#[aidoku_test]
fn search_finds_a_title() {
	let first = search(None, vec![]).entries.remove(0);
	let word = first
		.title
		.split(|c: char| !c.is_alphanumeric())
		.max_by_key(|word| word.len())
		.unwrap()
		.to_lowercase();
	let found = search(Some(&word), vec![]);
	assert!(found.entries.iter().any(|manga| manga.key == first.key));
	let nothing = search(Some("zzqxwv"), vec![]);
	assert!(nothing.entries.is_empty() && !nothing.has_next_page);
	// A pasted link finds that series alone
	let pasted = search(Some(&format!("{BASE_URL}{}", first.key)), vec![]);
	assert_eq!(keys(&pasted), [first.key.as_str()]);
	assert!(!pasted.has_next_page);
}

#[aidoku_test]
fn sorts_change_the_order() {
	let az = search(None, vec![sort(5)]);
	let za = search(None, vec![sort(6)]);
	assert_ne!(keys(&az), keys(&za));
	assert_ne!(
		keys(&search(None, vec![sort(4)])),
		keys(&search(None, vec![]))
	);
}

#[aidoku_test]
fn filters_narrow_results() {
	let all = search(None, vec![]);
	let tagged = search(None, vec![genres(&[GENRE])]);
	assert!(!tagged.entries.is_empty());
	assert_ne!(keys(&tagged), keys(&all));
	// A tapped tag sends its name, which finds the same series
	assert_eq!(
		keys(&search(None, vec![select("genre", GENRE_NAME)])),
		keys(&tagged)
	);
	let typed = search(None, vec![select("type", TYPE)]).entries;
	assert!(!typed.is_empty());
	assert_eq!(details(typed[0].clone()).viewer, Viewer::Webtoon);
	let by_status = search(None, vec![select("status", STATUS)]).entries;
	assert!(!by_status.is_empty());
	assert_eq!(details(by_status[0].clone()).status, MangaStatus::Completed);
	// "All" sends nothing
	assert_eq!(keys(&search(None, vec![select("type", "")])), keys(&all));
}

#[aidoku_test]
fn details_chapters_and_pages_load() {
	let (first, _) = with_chapters();
	let manga = source().get_manga_update(first, true, true).unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.cover.is_some());
	assert!(
		manga
			.description
			.as_deref()
			.is_none_or(|description| !description.is_empty())
	);
	assert!(manga.status != MangaStatus::Unknown);
	assert!(manga.tags.as_ref().is_some_and(|tags| !tags.is_empty()));
	let chapters = manga.chapters.clone().unwrap();
	assert!(!chapters.is_empty());
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.key.starts_with(&manga.key))
	);
	// Newest and oldest chapters, read with only their keys as the app may pass them
	for chapter in [chapters.first(), chapters.last()].into_iter().flatten() {
		let pages = source()
			.get_page_list(
				Manga {
					key: manga.key.clone(),
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
			|page| matches!(&page.content, PageContent::Url(url, _) if url.starts_with("https://"))
		));
	}
}

#[aidoku_test]
fn home_follows_the_site() {
	let home = source().get_home().unwrap();
	let titles: Vec<&str> = home
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	assert!(titles.contains(&"Featured Series"), "{titles:?}");
	let mut unique = titles.clone();
	unique.sort_unstable();
	unique.dedup();
	assert_eq!(unique.len(), titles.len(), "rows repeat");
	let recent = home
		.components
		.iter()
		.find(|component| component.title.as_deref() == Some("Recently Updated"))
		.unwrap();
	let HomeComponentValue::MangaChapterList { entries, .. } = &recent.value else {
		panic!("recently updated is a chapter list");
	};
	let entry = &entries[0];
	let chapters = source()
		.get_manga_update(entry.manga.clone(), false, true)
		.unwrap()
		.chapters
		.unwrap();
	assert!(
		chapters
			.iter()
			.any(|chapter| chapter.key == entry.chapter.key)
	);
}

#[aidoku_test]
fn deep_links_open_series_and_chapters() {
	let (first, mut chapters) = with_chapters();
	let chapter = chapters.remove(0);
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}{}", first.key))
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: first.key.clone()
		})
	);
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}{}", chapter.key))
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: first.key,
			key: chapter.key
		})
	);
	// The home page and the catalogue are no series
	assert_eq!(
		source().handle_deep_link(format!("{BASE_URL}/")).unwrap(),
		None
	);
	assert_eq!(
		source()
			.handle_deep_link(format!("{BASE_URL}/?browse=1"))
			.unwrap(),
		None
	);
}

#[aidoku_test]
fn an_unknown_series_fails() {
	let manga = Manga {
		key: "/zzqxwv-no-such-series".into(),
		..Default::default()
	};
	assert!(source().get_manga_update(manga, true, true).is_err());
}

#[aidoku_test]
fn novel_chapters_are_image_strips() {
	let (manga, chapter) = search(None, vec![select("type", "novel")])
		.entries
		.into_iter()
		.find_map(|manga| {
			let chapter = source()
				.get_manga_update(manga.clone(), false, true)
				.unwrap()
				.chapters?
				.pop()?;
			Some((manga, chapter))
		})
		.unwrap();
	assert_eq!(details(manga.clone()).viewer, Viewer::Webtoon);
	let pages = source()
		.get_page_list(
			Manga {
				key: manga.key,
				..Default::default()
			},
			Chapter {
				key: chapter.key,
				..Default::default()
			},
		)
		.unwrap();
	assert!(!pages.is_empty());
	assert!(pages.iter().all(
		|page| matches!(&page.content, PageContent::Url(url, _) if url.starts_with("https://"))
	));
}
