#![expect(clippy::unwrap_used)]

use super::*;
use aidoku::{
	Chapter, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponentValue, LinkValue,
	Manga, MangaPageResult, MangaStatus, PageContent, Viewer,
	alloc::{String, Vec, format, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

// The site's genre id for Action, from res/filters.json.
const GENRE: &str = "action";

// A home page cut down to one card per row, in the site's markup and order. `{aspect}` is the
// featured slides' shape: covers on this site, banners on others.
const HOME: &str = r#"
<a href="/series/featured/" title="Featured" class="splide__slide"><div class="aspect-{aspect}" style="background-image: url(https://img.test/featured.avif);"></div></a>
<div><div><h2>Pinned Series</h2></div><div><div class="latest-poster"><a href="/series/pinned/" title="Pinned"></a><a href="/chapter/p-1" title="Chapter 1" d="1 day ago"></a></div></div></div>
<div><div><h2>Bulk Discounted Series</h2></div><div><button title="Bulk"><a href="/series/bulk/" title="Bulk"></a></button></div></div>
<div><div><h2>Latest Updates</h2></div><div>
<div class="latest-poster"><a href="/series/new/" title="New"></a><a href="/chapter/n-2" title="Chapter 2" d="1 hour ago"><img alt="Coin"></a><a href="/chapter/n-1" title="Chapter 1" d="2 hours ago"></a></div>
</div></div>
<div><div><h2>Trending</h2></div><div><button title="Hot"><a href="/series/hot/" title="Hot"><div style="background-image: url(https://img.test/hot.avif);"></div></a></button></div></div>
<div><div><h2>Editor's Choice</h2></div><div><div class="editor-pick-button" title="Pick" data-id="pick" data-img="pick.avif" data-description="QSBwbGFpbiBkZXNjcmlwdGlvbi4="></div></div></div>
<div><div><h2>Editor's Choice</h2></div><div><div class="editor-pick-button" title="Pick" data-id="pick" data-img="pick.avif" data-description="QSBwbGFpbiBkZXNjcmlwdGlvbi4="></div></div></div>
<div><h2>Sync Local Library</h2></div>
"#;

fn home(aspect: &str) -> aidoku::HomeLayout {
	let html = Html::parse(HOME.replace("{aspect}", aspect)).unwrap();
	let source = GenzToons::new();
	source.parse_home(&source.params(), &html)
}

fn source() -> Keyoapp<GenzToons> {
	Keyoapp::new()
}

fn search(query: Option<&str>, filters: Vec<FilterValue>) -> MangaPageResult {
	source()
		.get_search_manga_list(query.map(String::from), 1, filters)
		.unwrap()
}

fn multi(id: &str, included: &[&str], excluded: &[&str]) -> FilterValue {
	FilterValue::MultiSelect {
		id: id.into(),
		included: included.iter().map(|value| (*value).into()).collect(),
		excluded: excluded.iter().map(|value| (*value).into()).collect(),
	}
}

fn details(manga: Manga) -> Manga {
	source().get_manga_update(manga, true, false).unwrap()
}

#[aidoku_test]
fn review_home_keeps_the_site_rows() {
	let layout = home("[0.74/1]");
	let rows: Vec<Option<&str>> = layout
		.components
		.iter()
		.map(|component| component.title.as_deref())
		.collect();
	// Pinned repeats Latest Updates here, bulk discounts sell coin chapters, Editor's Choice
	// repeats for the mobile layout, and Sync Local Library has no cards.
	assert_eq!(
		rows,
		[
			None,
			Some("Latest Updates"),
			Some("Trending"),
			Some("Editor's Choice")
		]
	);
	let values: Vec<&HomeComponentValue> = layout.components.iter().map(|c| &c.value).collect();
	assert!(
		matches!(values[0], HomeComponentValue::Scroller { entries, .. } if entries[0].title == "Featured")
	);
	let HomeComponentValue::MangaChapterList {
		page_size, entries, ..
	} = values[1]
	else {
		panic!("latest updates is a chapter list");
	};
	assert_eq!(*page_size, Some(5));
	// The coin chapter needs a login, so the row shows the newest free one.
	assert_eq!(entries[0].chapter.key, "/chapter/n-1");
	assert!(
		matches!(values[2], HomeComponentValue::Scroller { entries, .. } if entries[0].subtitle.is_none())
	);
	let HomeComponentValue::BigScroller { entries, .. } = values[3] else {
		panic!("editor's choice is a big scroller");
	};
	assert_eq!(entries[0].key, "/series/pick/");
	assert_eq!(
		entries[0].description.as_deref(),
		Some("A plain description.")
	);
}

#[aidoku_test]
fn review_banners_open_their_series() {
	let layout = home("[2/1]");
	let HomeComponentValue::ImageScroller {
		links,
		width,
		height,
		..
	} = &layout.components[0].value
	else {
		panic!("a banner slider is an image scroller");
	};
	assert_eq!((*width, *height), (Some(400), Some(200)));
	assert_eq!(
		links[0].image_url.as_deref(),
		Some("https://img.test/featured.avif")
	);
	assert!(
		matches!(&links[0].value, Some(LinkValue::Manga(manga)) if manga.key == "/series/featured/")
	);
}

#[aidoku_test]
fn review_filters_are_include_only() {
	// As on the site, no filter can exclude.
	let filters = include_str!("../res/filters.json");
	assert_eq!(filters.matches("\"canExclude\": false").count(), 3);
	assert!(!filters.contains("\"canExclude\": true"));
}

#[aidoku_test]
fn search_lists_the_catalogue_on_one_page() {
	let all = search(None, vec![]);
	assert!(all.entries.len() > 50);
	assert!(!all.has_next_page);
	// Cards carry only what the list shows.
	assert!(all.entries.iter().all(|manga| {
		manga.key.starts_with("/series/") && manga.cover.is_some() && manga.description.is_none()
	}));
}

#[aidoku_test]
fn search_finds_a_title() {
	let first = search(None, vec![]).entries.remove(0);
	// Letters and digits only: the cards' titles carry quotes the query needn't.
	let word = first
		.title
		.split(|c: char| !c.is_alphanumeric())
		.max_by_key(|word| word.len())
		.unwrap()
		.to_lowercase();
	let found = search(Some(&word), vec![]);
	assert!(found.entries.iter().any(|manga| manga.key == first.key));
	assert!(search(Some("zzqxwv"), vec![]).entries.is_empty());
	// A pasted link finds that series alone.
	let pasted = search(Some(&format!("{BASE_URL}{}", first.key)), vec![]);
	assert_eq!(pasted.entries, [first]);
}

#[aidoku_test]
fn genre_filter_needs_every_genre() {
	let all = search(None, vec![]).entries.len();
	let action = search(None, vec![multi("genre", &[GENRE], &[])]).entries;
	assert!(!action.is_empty() && action.len() < all);
	let tags = details(action[0].clone()).tags.unwrap();
	assert!(tags.iter().any(|tag| tag.eq_ignore_ascii_case(GENRE)));
	// As on the site, a series needs every chosen genre.
	let both = search(None, vec![multi("genre", &[GENRE, "fantasy"], &[])]).entries;
	assert!(!both.is_empty() && both.len() < action.len());
	assert!(both.iter().all(|manga| action.contains(manga)));
}

#[aidoku_test]
fn type_and_status_filters_narrow_results() {
	let all = search(None, vec![]).entries.len();
	let manga = search(None, vec![multi("type", &["manga"], &[])]).entries;
	assert!(!manga.is_empty() && manga.len() < all);
	assert_eq!(details(manga[0].clone()).viewer, Viewer::RightToLeft);
	let completed = search(None, vec![multi("status", &["completed"], &[])]).entries;
	assert!(!completed.is_empty() && completed.len() < all);
	assert_eq!(details(completed[0].clone()).status, MangaStatus::Completed);
}

#[aidoku_test]
fn details_chapters_and_pages_load() {
	let first = search(None, vec![]).entries.remove(0);
	let manga = source().get_manga_update(first, true, true).unwrap();
	assert!(!manga.title.is_empty());
	assert!(manga.cover.is_some());
	// Some series have no synopsis on the site at all.
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
			.all(|chapter| chapter.key.starts_with("/chapter/"))
	);
	assert!(
		chapters
			.iter()
			.all(|chapter| chapter.date_uploaded.is_some())
	);
	// Newest and oldest chapters, read with only their keys as the app may pass them.
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
	let HomeComponentValue::Scroller { entries, .. } = &home.components[0].value else {
		panic!("the featured slider comes first");
	};
	assert!(home.components[0].title.is_none() && entries.len() >= 3);
	let titles: Vec<&str> = home
		.components
		.iter()
		.filter_map(|component| component.title.as_deref())
		.collect();
	let position = |title: &str| titles.iter().position(|found| *found == title).unwrap();
	assert!(position("Latest Updates") < position("Trending"));
	assert!(position("Trending") < position("Recently Added"));
	let mut unique = titles.clone();
	unique.dedup();
	assert_eq!(unique.len(), titles.len(), "rows repeat");
	assert!(!titles.contains(&"Bulk Discounted Series"));
	let row = |title: &str| &home.components[position(title) + 1].value;
	assert!(!titles.contains(&"Pinned Series"), "pinned repeats latest");
	let HomeComponentValue::BigScroller { entries: picks, .. } = row("Editor's Choice") else {
		panic!("editor's choice is a big scroller");
	};
	let pick = &picks[0];
	assert!(
		pick.cover.is_some()
			&& pick
				.description
				.as_deref()
				.is_some_and(|text| !text.is_empty())
	);
	assert!(
		search(None, vec![])
			.entries
			.iter()
			.any(|manga| manga.key == pick.key)
	);
	let HomeComponentValue::MangaChapterList {
		page_size, entries, ..
	} = row("Latest Updates")
	else {
		panic!("latest updates is a chapter list");
	};
	assert_eq!(*page_size, Some(5));
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
	let first = search(None, vec![]).entries.remove(0);
	let manga = source()
		.get_manga_update(first.clone(), false, true)
		.unwrap();
	let chapter = manga.chapters.unwrap().remove(0);
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
	// The catalogue itself is no series.
	for path in ["/series/", "/series/?genre=action"] {
		assert_eq!(
			source()
				.handle_deep_link(format!("{BASE_URL}{path}"))
				.unwrap(),
			None
		);
	}
}

#[aidoku_test]
fn an_unknown_series_fails() {
	let manga = Manga {
		key: "/series/zzqxwv-no-such-series/".into(),
		..Default::default()
	};
	// The site redirects it to a not-found page.
	assert!(
		source()
			.get_manga_update(manga.clone(), true, true)
			.is_err()
	);
	assert!(source().get_manga_update(manga, false, true).is_err());
}
