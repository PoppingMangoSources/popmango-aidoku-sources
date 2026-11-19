#![expect(clippy::unwrap_used)]

use crate::{
	Params, Row,
	helpers::{age, comic_id, details, narrowed, series_path, slugify},
	home::fill,
};
use aidoku::{
	ContentRating, HomeComponentValue, Manga, MangaStatus, Viewer,
	alloc::{String, Vec, string::ToString, vec},
	imports::html::Html,
};
use aidoku_test::aidoku_test;

const TOONZ: Params = Params {
	base_url: "https://toonz.to",
	catalog: "comics",
	series_path: "comic",
	series_types: &["manhwa", "manga", "manhua", "webtoon", "comic", "western"],
	rating: "erotica",
	type_count: 5,
	home_pages: &[],
	home_rows: &[],
};

const LEWD: Params = Params {
	base_url: "https://lewdmanhwa.com",
	catalog: "webtoons",
	series_path: "webtoon",
	series_types: &["webtoon"],
	rating: "pornographic",
	type_count: 5,
	home_pages: &[],
	home_rows: &[],
};

const SERIES_PAGE: &str = r#"<html><head><link rel="canonical" href="https://toonz.to/manhwa/example"/>
<meta property="og:image" content="https://toonz.to/uploads/covers/example.webp"/></head><body><main>
<span hidden data-comic-id="1069693"></span><img data-cover data-nsfw src="/uploads/covers/example_thumb.webp"/>
<h1>Example Series</h1><dl><div><dt>Type</dt><dd>Manhwa</dd></div><div><dt>Status</dt><dd>Completed</dd></div>
<div><dt>Author</dt><dd><a href="/author/a">First Writer</a><a href="/author/b">Second</a><a href="/author/a">First Writer</a></dd></div></dl>
<p class="whitespace-pre-line">A *plain* summary.</p><div><p>Genres</p><a href="/genre/romance">Romance</a><a href="/genre/drama">Drama</a></div>
</main></body></html>"#;

// Badges in place of the table, and a related card's badges after them.
const BADGE_PAGE: &str = r#"<html><body><span hidden data-comic-id="541"></span><img data-cover src="/c.webp"/>
<h1>Badged</h1><a href="/author/jiong"><span data-slot="badge">Jiong</span></a><a href="/genre/yaoi"><span data-slot="badge">Yaoi</span></a>
<span data-slot="badge">Ongoing</span><span data-slot="badge">Manga</span><p class="whitespace-pre-line">Summary.</p>
<div><span data-slot="badge">Completed</span><span data-slot="badge">Manhwa</span></div></body></html>"#;

const MANHWA_PAGE: &str = r#"<html><body><main>
<section aria-label="Featured"><article><p>Top rated this month</p><h2><a href="/manhwa/low">Low</a></h2><p>Low summary.</p>
<div><a href="/genre/romance">Romance</a></div><dl><div><dt>Rating</dt><dd><b>4.2</b> rating</dd></div></dl><img data-cover src="/low.webp"/></article>
<article><h2><a href="/manhwa/high">High</a></h2><p>High summary.</p><div><a href="/genre/smut">Smut</a><a href="/genre/drama">Drama</a></div>
<dl><div><dt>Rating</dt><dd><b>5.0</b> rating</dd></div></dl><img data-cover data-nsfw src="/high.webp"/></article></section>
<section><h2>Trending</h2><div role="tabpanel"><a href="/manhwa/today"><img data-cover src="/t.webp" alt="Today"/><h3>Today</h3><span>Ch. 4</span></a></div>
<div role="tabpanel" hidden><a href="/manhwa/week"><img data-cover src="/w.webp" alt="Week"/><h3>Week</h3></a></div></section>
<section><h2>New Titles</h2><a href="/manhwa/older"><img data-cover src="/o.webp"/><h3>Older</h3><span>Ch. 2.1</span><span>2h ago</span></a></section>
</main></body></html>"#;

const MANGA_PAGE: &str = r#"<html><body><main>
<section><h2>New Titles</h2><a href="/manga/newer"><img data-cover src="/n.webp"/><h3>Newer</h3><span>Ch. 1</span><span>5m ago</span></a>
<a href="/manga/older"><img data-cover src="/o.webp"/><h3>Older</h3><span>2h ago</span></a></section>
</main></body></html>"#;

const LATEST_PAGE: &str = r#"<html><body><main><article><a href="/manga/kawaisou"><div><img src="/k.webp" alt="Kawaisou"/></div></a>
<div><div><a href="/manga/kawaisou">Kawaisou</a></div><div><span><span>manga</span></span><span><span>·</span><span>CH.<!-- -->15.5</span></span></div>
<p>Summary.</p><p>2w ago</p></div></article></main></body></html>"#;

// The hero, a ranked row and cards whose link is an empty overlay beside the cover.
const LEWD_HOME: &str = r#"<html><body>
<section><div aria-hidden="false"><img alt="" src="/bg.webp"/><div><a aria-label="Beast" href="/webtoon/beast"><div><picture>
<img alt="Beast" src="/uploads/covers/beast-hd.webp"/></picture><div>manhwa</div></div></a><div><span data-slot="badge">#1 Featured</span>
<a href="/webtoon/beast"><h2>Beast</h2></a><div><span>3.3</span><span data-slot="badge">ongoing</span></div><p>Beast summary.</p>
<a href="/webtoon/beast"><button>Read Now</button></a></div></div></div></section>
<section><h2>Trending Now</h2><a href="/webtoon/beast"><div><img alt="Beast" data-cover src="/b.webp"/><div>1</div></div><h3>Beast</h3></a>
<a href="/webtoon/farm-429h7"><div><img alt="Farm" data-cover src="/f.webp"/><div>2</div></div><h3>Farm</h3></a></section>
<section><h2>Latest Updates</h2><div><a aria-label="Divorce" href="/webtoon/divorce"></a><div><img alt="Divorce" data-cover src="/d.webp"/>
<div><span>Manhwa</span></div></div><div><h3>Divorce</h3><div><span>223</span></div><a href="/webtoon/divorce/chapter/17-wad21">Ch. <!-- -->17</a></div></div></section>
</body></html>"#;

fn page(html: &str, base: &str) -> Option<aidoku::imports::html::Document> {
	Some(Html::parse_with_url(html, base).unwrap())
}

#[aidoku_test]
fn review_home_rows_read_their_own_cards() {
	let pages = [
		page(MANHWA_PAGE, TOONZ.base_url),
		page(MANGA_PAGE, TOONZ.base_url),
		page(LATEST_PAGE, TOONZ.base_url),
	];
	let value = |row: Row| fill(&TOONZ, row, &pages);

	let Some(HomeComponentValue::BigScroller { entries, .. }) = value(Row::Featured(&[0, 1]))
	else {
		panic!("featured is not a big scroller");
	};
	let featured: Vec<_> = entries.iter().map(|manga| manga.key.as_str()).collect();
	assert_eq!(featured, ["high", "low"]);
	assert_eq!(
		entries[0].tags.as_deref(),
		Some(&["Drama".to_string(), "Smut".into()][..])
	);
	assert_eq!(entries[0].content_rating, ContentRating::NSFW);
	assert_eq!(entries[0].description.as_deref(), Some("High summary."));

	let Some(HomeComponentValue::Scroller { entries, .. }) =
		value(Row::Newest(&[0, 1], "New Titles"))
	else {
		panic!("new titles is not a scroller");
	};
	let new: Vec<_> = entries.iter().map(|link| link.title.as_str()).collect();
	assert_eq!(new, ["Newer", "Older"]);

	let Some(HomeComponentValue::MangaChapterList {
		entries, page_size, ..
	}) = value(Row::Updates(2))
	else {
		panic!("recently updated is not a chapter list");
	};
	assert_eq!(page_size, Some(5));
	assert_eq!(entries[0].manga.key, "kawaisou");
	assert_eq!(entries[0].chapter.key, "15.5");
	assert_eq!(entries[0].chapter.chapter_number, Some(15.5));
	assert!(entries[0].chapter.date_uploaded.is_some());

	let Some(HomeComponentValue::Scroller { entries, .. }) = value(Row::Tab(0, "Trending")) else {
		panic!("trending is not a scroller");
	};
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].title, "Today");
	assert!(entries[0].subtitle.is_none());
	assert!(
		value(Row::Tab(1, "Trending")).is_none(),
		"an empty row must not borrow another page's cards"
	);
}

#[aidoku_test]
fn review_lewd_home_rows() {
	let pages = [page(LEWD_HOME, LEWD.base_url)];
	let value = |row: Row| fill(&LEWD, row, &pages);

	let Some(HomeComponentValue::BigScroller { entries, .. }) = value(Row::Hero(0)) else {
		panic!("the hero is not a big scroller");
	};
	assert_eq!(entries.len(), 1, "only slide headings link a series");
	assert_eq!(entries[0].key, "beast");
	assert_eq!(
		entries[0].cover.as_deref(),
		Some("https://lewdmanhwa.com/uploads/covers/beast-hd.webp")
	);
	assert_eq!(entries[0].description.as_deref(), Some("Beast summary."));
	assert_eq!(entries[0].status, MangaStatus::Ongoing);

	let Some(HomeComponentValue::Scroller { entries, .. }) = value(Row::Cards(0, "Trending Now"))
	else {
		panic!("trending now is not a scroller");
	};
	let keys: Vec<_> = entries.iter().map(|link| link.title.as_str()).collect();
	assert_eq!(keys, ["Beast", "Farm"]);

	let Some(HomeComponentValue::MangaChapterList { entries, .. }) =
		value(Row::Chapters(0, "Latest Updates"))
	else {
		panic!("latest updates is not a chapter list");
	};
	assert_eq!(entries.len(), 1);
	assert_eq!(entries[0].manga.key, "divorce");
	assert_eq!(entries[0].manga.title, "Divorce");
	assert_eq!(entries[0].chapter.key, "17-wad21");
	assert_eq!(entries[0].chapter.chapter_number, Some(17.0));
	assert_eq!(
		entries[0].chapter.url.as_deref(),
		Some("https://lewdmanhwa.com/webtoon/divorce/chapter/17-wad21")
	);
}

#[aidoku_test]
fn review_details_read_the_series_page() {
	let page = Html::parse_with_url(SERIES_PAGE, TOONZ.base_url).unwrap();
	let mut manga = Manga {
		key: "example".into(),
		..Default::default()
	};
	details(&page, &mut manga).unwrap();
	assert_eq!(manga.title, "Example Series");
	assert_eq!(
		manga.cover.as_deref(),
		Some("https://toonz.to/uploads/covers/example.webp")
	);
	assert_eq!(
		manga.url.as_deref(),
		Some("https://toonz.to/manhwa/example")
	);
	assert_eq!(
		manga.authors,
		Some(vec!["First Writer".into(), "Second".into()])
	);
	assert_eq!(manga.tags, Some(vec!["Drama".into(), "Romance".into()]));
	assert_eq!(manga.status, MangaStatus::Completed);
	assert_eq!(manga.viewer, Viewer::Webtoon);
	assert_eq!(manga.content_rating, ContentRating::NSFW);
	assert_eq!(manga.description.as_deref(), Some(r"A \*plain\* summary\."));
	assert_eq!(comic_id(&page).unwrap(), "1069693");
}

#[aidoku_test]
fn review_details_read_badges() {
	let page = Html::parse_with_url(BADGE_PAGE, LEWD.base_url).unwrap();
	let mut manga = Manga::default();
	details(&page, &mut manga).unwrap();
	assert_eq!(manga.title, "Badged");
	assert_eq!(manga.authors, Some(vec!["Jiong".into()]));
	assert_eq!(manga.tags, Some(vec!["Yaoi".into()]));
	assert_eq!(manga.status, MangaStatus::Ongoing);
	assert_eq!(manga.viewer, Viewer::RightToLeft);
	assert_eq!(manga.content_rating, ContentRating::Safe);
	assert_eq!(comic_id(&page).unwrap(), "541");
}

#[aidoku_test]
fn review_links_and_ages() {
	let link = |params: &Params, path: &str| series_path(params, path);
	assert_eq!(
		link(&TOONZ, "/manhwa/solo-leveling"),
		Some(("solo-leveling".into(), None))
	);
	assert_eq!(
		link(&TOONZ, "/manga/x/chapter/84.02?page=2"),
		Some(("x".into(), Some("84.02".into())))
	);
	assert_eq!(link(&TOONZ, "/manhwa/browse"), None);
	assert_eq!(link(&TOONZ, "/manga/trending"), None);
	assert_eq!(link(&TOONZ, "/genre/action"), None);
	assert_eq!(link(&TOONZ, "/webtoon/x"), Some(("x".into(), None)));
	assert_eq!(link(&LEWD, "/manhwa/x"), None);
	assert_eq!(
		link(&LEWD, "/webtoon/rockstar-vwe1q/chapter/45-wad21"),
		Some(("rockstar-vwe1q".into(), Some("45-wad21".into())))
	);
	assert_eq!(age("just now"), Some(0));
	assert_eq!(age("32m ago"), Some(32 * 60));
	assert_eq!(age("3mo ago"), Some(3 * 2_592_000));
	assert_eq!(age("Ch. 4"), None);
	for (name, slug) in [
		("Chugong", "chugong"),
		("13th month's dawn", "13th-months-dawn"),
		("2:10 AM Animation", "210-am-animation"),
		("3rd_R.I.O (써드리오)", "3rd-rio"),
		("1000+10 (atsuyu)", "100010-atsuyu"),
	] {
		assert_eq!(slugify(name), slug);
	}
}

#[aidoku_test]
fn review_type_selections() {
	let types = |values: &[&str]| narrowed(values.iter().map(|v| String::from(*v)).collect(), 3);
	assert_eq!(types(&[]), ["none"]);
	assert_eq!(types(&["manga"]), ["manga"]);
	assert!(types(&["manga", "manhwa", "manhua"]).is_empty());
}
