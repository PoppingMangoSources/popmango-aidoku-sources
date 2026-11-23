#![expect(clippy::unwrap_used)]

use crate::{
	Params, Row,
	helpers::{age, comic_id, details, narrowed, parse_series_path, slugify},
	home::fill,
	models::{ChapterItem, ReaderImage},
	rsc::reader_images,
};
use aidoku::{
	ContentRating, HomeComponentValue, Manga, MangaStatus, PageContent, Viewer,
	alloc::{String, Vec, format, string::ToString, vec},
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

const TOONX: Params = Params {
	base_url: "https://toonx.to",
	catalog: "comics",
	series_path: "comic",
	series_types: &["comic", "adult"],
	rating: "pornographic",
	type_count: 0,
	home_pages: &[],
	home_rows: &[],
};

const SERIES_PAGE: &str = r#"<html><head><link rel="canonical" href="https://toonz.to/manhwa/example"/>
<meta property="og:image" content="https://toonz.to/uploads/covers/example.webp"/></head><body><main>
<span hidden data-comic-id="1069693"></span><img data-cover data-nsfw src="/uploads/covers/example_thumb.webp"/>
<h1>Example Series</h1><dl><div><dt>Type</dt><dd>Manhwa</dd></div><div><dt>Status</dt><dd>Completed</dd></div>
<div><dt>Author</dt><dd><a href="/author/a">First Writer</a><a href="/author/b">Second</a><a href="/author/a">First Writer</a></dd></div></dl>
<p class="whitespace-pre-line">A *formatted* summary.

[More information](https://example.com/).</p><div><p>Genres</p><a href="/genre/romance">Romance</a><a href="/genre/drama">Drama</a></div>
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
<a href="/manga/older"><img data-cover src="/o.webp"/><h3>Older</h3><span>2h ago</span></a>
<a href="/manga/no-date"><img data-cover src="/u.webp"/><h3>No Date</h3></a></section>
</main></body></html>"#;

const LATEST_PAGE: &str = r#"<html><body><main><article><a href="/manga/kawaisou"><div><img src="/k.webp" alt="Kawaisou"/></div></a>
<div><div><a href="/manga/kawaisou">Kawaisou</a></div><div><span><span>manga</span></span><span><span>·</span><span>CH.<!-- -->15.5</span></span></div>
<p>Summary.</p><p>2w ago</p></div></article>
<article><a href="/manga/undated"><div><img src="/u.webp" alt="Undated"/></div><span>CH. 3</span></a></article></main></body></html>"#;

// The hero, a ranked row and cards whose link is an empty overlay beside the cover.
const LEWD_HOME: &str = r#"<html><body>
<section><div aria-hidden="false"><img alt="" src="/bg.webp"/><div><a aria-label="Beast" href="/webtoon/beast"><div><picture>
<img alt="Beast" src="/uploads/covers/beast-hd.webp"/></picture><div>manhwa</div></div></a><div><span data-slot="badge">#1 Featured</span>
<a href="/webtoon/beast"><h2>Beast</h2></a><div><span>3.3</span><span data-slot="badge">ongoing</span></div><p>Beast summary.</p>
<a href="/webtoon/beast"><button>Read Now</button></a></div></div>
<div aria-hidden="true"><div><a aria-label="Farm" href="/webtoon/farm-429h7"><div><div>manhwa</div></div></a><div>
<a href="/webtoon/farm-429h7"><h2>Farm</h2></a><p>A choice between home / the unknown.</p></div></div></div>
<div aria-hidden="true"><div><div></div><div><a href="/webtoon/fields"><h2>Fields</h2></a>
<p>Another Title! / Alternate Title! /  /  /  ! Someone returns to the fields.Original WebtoonOfficial Tr</p></div></div></div>
<div aria-hidden="true"><div><div></div><div><a href="/webtoon/forest"><h2>Forest</h2></a><p>Another Forest /  /  !</p></div></div></div>
<div aria-hidden="true"><div><div></div><div><a href="/webtoon/creek"><h2>Creek</h2></a></div></div></div>
<div aria-hidden="true"><div><div></div><div><a href="/webtoon/garden"><h2>Garden</h2></a></div></div></div></section>
<section><a href="/webtoon/unrelated"><h2>Not Featured</h2></a></section>
<section><h2>Trending Now</h2><a href="/webtoon/beast"><div><img alt="Beast" data-cover src="/b.webp"/><div>1</div></div><h3>Beast</h3></a>
<a href="/webtoon/farm-429h7"><div><img alt="Farm" data-cover src="/f.webp"/><div>2</div></div><h3>Farm</h3></a></section>
<section><h2>Latest Updates</h2><div><a aria-label="Divorce" href="/webtoon/divorce"></a><div><img alt="Divorce" data-cover src="/d.webp"/>
<div><span>Manhwa</span></div></div><div><h3>Divorce</h3><div><span>223</span></div><a href="/webtoon/divorce/chapter/17-wad21">Ch. <!-- -->17</a><a href="/webtoon/divorce/chapter/17-wad21">Read</a></div></div></section>
<script>self.__next_f.push([1,"1a:[\"$\",\"$L3\",null,{\"comics\":[{\"slug\":\"fields\",\"cover\":\"covers/fields.webp\"},{\"slug\":\"forest\",\"cover\":\"covers/forest.webp\"},{\"slug\":\"creek\",\"cover\":\"covers/creek.webp\"},{\"slug\":\"garden\",\"cover\":\"covers/garden.webp\"},{\"slug\":\"unrelated\",\"cover\":\"covers/unrelated.webp\"}]}]\n"])</script>
</body></html>"#;

fn page(html: &str, base: &str) -> Option<aidoku::imports::html::Document> {
	Some(Html::parse_with_url(html, base).unwrap())
}

fn top_rated_page(kind: &str) -> String {
	let mut html = String::from(r#"<section aria-label="Featured">"#);
	let mut series = Vec::new();
	for number in 1..=5 {
		let slug = format!("{kind}-{number}");
		html.push_str(&format!(
			r#"<article><h2><a href="/{kind}/{slug}">{slug}</a></h2><p>Summary</p><dl><div><dt>Rating</dt><dd>5.0 rating</dd></div></dl>"#
		));
		if number <= 2 {
			html.push_str(&format!(r#"<img data-cover src="/{slug}_thumb.webp">"#));
		}
		html.push_str("</article>");
		series.push(format!(
			r#"{{"slug":"{slug}","description":"Quote: \"hello\"","cover":"covers/{slug}-hd.webp","contentRating":"erotica"}}"#
		));
	}
	html.push_str("</section>");
	let flight = format!(
		r#"3d:["$","$L6b",null,{{"slides":[{}]}}]"#,
		series.join(",")
	);
	let (first, second) = flight.split_at(flight.find("slides").unwrap() + 3);
	for chunk in [first, second] {
		let chunk = serde_json::to_string(&(1, chunk)).unwrap();
		html.push_str(&format!("<script>self.__next_f.push({chunk})</script>"));
	}
	html
}

#[aidoku_test]
fn review_top_rated_keeps_five_titles_from_each_type() {
	let pages = [
		page(&top_rated_page("manhwa"), TOONZ.base_url),
		page(&top_rated_page("manga"), TOONZ.base_url),
	];
	let Some(HomeComponentValue::BigScroller { entries, .. }) =
		fill(&TOONZ, Row::Featured(&[0, 1]), &pages)
	else {
		panic!("top rated is not a big scroller");
	};
	assert_eq!(entries.len(), 10);
	for (index, entry) in entries.iter().enumerate() {
		let kind = if index < 5 { "manhwa" } else { "manga" };
		let number = index % 5 + 1;
		assert_eq!(entry.key, format!("{kind}-{number}"));
		assert!(entry.cover.is_some());
	}
}

#[aidoku_test]
fn review_toonx_top_rated_recovers_later_covers() {
	let pages = [page(&top_rated_page("adult"), TOONX.base_url)];
	let Some(HomeComponentValue::BigScroller { entries, .. }) =
		fill(&TOONX, Row::Featured(&[0]), &pages)
	else {
		panic!("top rated is not a big scroller");
	};
	assert_eq!(entries.len(), 5);
	assert_eq!(entries[4].key, "adult-5");
	assert_eq!(
		entries[4].cover.as_deref(),
		Some("https://toonx.to/uploads/covers/adult-5-hd.webp")
	);
	assert_eq!(entries[4].content_rating, ContentRating::NSFW);
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
	assert_eq!(new, ["Newer", "Older", "No Date"]);

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
	assert_eq!(entries[1].chapter.key, "3");
	assert_eq!(entries[1].chapter.date_uploaded, None);

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
	let keys: Vec<_> = entries.iter().map(|manga| manga.key.as_str()).collect();
	assert_eq!(
		keys,
		["beast", "farm-429h7", "fields", "forest", "creek", "garden"],
		"only slide headings link a series"
	);
	assert_eq!(
		entries[0].cover.as_deref(),
		Some("https://lewdmanhwa.com/uploads/covers/beast-hd.webp")
	);
	assert_eq!(entries[0].description.as_deref(), Some("Beast summary."));
	assert_eq!(entries[0].status, MangaStatus::Ongoing);
	assert_eq!(
		entries[1].cover.as_deref(),
		Some("https://lewdmanhwa.com/f.webp"),
		"a slide without its own cover takes its card's"
	);
	assert_eq!(
		entries[1].description.as_deref(),
		Some("A choice between home / the unknown.")
	);
	assert_eq!(
		entries[2].description.as_deref(),
		Some("Someone returns to the fields."),
		"alternate titles and the cut credit label are not the synopsis"
	);
	assert_eq!(entries[3].description, None);
	assert_eq!(
		entries[5].cover.as_deref(),
		Some("https://lewdmanhwa.com/uploads/covers/garden.webp")
	);
	assert!(entries.iter().all(|entry| entry.cover.is_some()));

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
	assert!(
		entries[0].chapter.url.is_none(),
		"the app opens home chapters by key"
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
	assert_eq!(
		manga.description.as_deref(),
		Some("A *formatted* summary.\n\n[More information](https://example.com/).")
	);
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
	let link = parse_series_path;
	assert_eq!(
		link(&TOONZ, "/manhwa/solo-leveling"),
		Some(("solo-leveling", None))
	);
	assert_eq!(
		link(&TOONZ, "/manga/x/chapter/84.02?page=2"),
		Some(("x", Some("84.02")))
	);
	assert_eq!(link(&TOONZ, "/manhwa?sort=popular"), None);
	assert_eq!(
		link(&TOONZ, "/manhwa/solo-leveling?ref=home#chapters"),
		Some(("solo-leveling", None))
	);
	assert_eq!(link(&TOONZ, "/manhwa/browse"), None);
	assert_eq!(link(&TOONZ, "/manga/trending"), None);
	assert_eq!(link(&TOONZ, "/genre/action"), None);
	assert_eq!(link(&TOONZ, "/webtoon/x"), Some(("x", None)));
	assert_eq!(link(&LEWD, "/manhwa/x"), None);
	assert_eq!(
		link(&LEWD, "/webtoon/rockstar-vwe1q/chapter/45-wad21"),
		Some(("rockstar-vwe1q", Some("45-wad21")))
	);
	assert_eq!(link(&TOONZ, "/manhwa/x/unknown"), None);
	assert_eq!(link(&TOONZ, "manhwa/x"), None);
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
		// Tapped genres are searched by slugified name; these are the dotted ones.
		("D.Va", "dva"),
		("Fantasy.Harem", "fantasyharem"),
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

#[aidoku_test]
fn review_chapter_keys_and_titles() {
	let chapter = |data: &str| {
		serde_json::from_str::<ChapterItem>(data)
			.unwrap()
			.into_chapter(&TOONZ, "example")
	};
	let dated = chapter(r#"{"chapterNumber":"12","title":"  12 Days Later  ","slug":"chapter-12-alt","date":"2026-11-22T14:30:06.000Z"}"#).unwrap();
	assert_eq!(dated.key, "12-alt");
	// The runner drops quoted format literals, so the UTC date is checked in the app.
	assert_eq!(dated.title.as_deref(), Some("12 Days Later"));
	let numbered =
		chapter(r#"{"chapterNumber":"0","title":"Chapter 0","slug":"chapter-0"}"#).unwrap();
	assert_eq!(numbered.key, "0");
	assert!(numbered.title.is_none());
	assert!(chapter(r#"{"chapterNumber":"0","slug":"chapter-"}"#).is_none());
}

#[aidoku_test]
fn review_reader_images_and_missing_pages() {
	let pages: Vec<_> = reader_images::<ReaderImage>(
		r#"0:{"images":["unrelated"]}
1:{"images":[{"filename":"second.webp","sortOrder":2},{"filename":"/first.webp","sortOrder":1}]}"#,
	)
	.unwrap()
	.into_iter()
	.map(
		|image| match image.into_page(TOONZ.base_url).unwrap().content {
			PageContent::Url(url, _) => url,
			_ => panic!("expected image url page"),
		},
	)
	.collect();
	assert_eq!(
		pages,
		[
			"https://toonz.to/uploads/second.webp",
			"https://toonz.to/uploads/first.webp",
		]
	);
	assert!(reader_images::<ReaderImage>(r#"{"images":[]}"#).is_err());
	assert!(reader_images::<ReaderImage>(r#"{"images":[{"sortOrder":1}]}"#).is_err());
	let blank = reader_images::<ReaderImage>(r#"{"images":[{"filename":" / "}]}"#)
		.unwrap()
		.into_iter()
		.next()
		.unwrap();
	assert!(blank.into_page(TOONZ.base_url).is_err());
}

#[aidoku_test]
fn review_mistagged_manga_viewer_overrides() {
	let viewer_for = |html: &str| {
		let page = Html::parse_with_url(html, TOONZ.base_url).unwrap();
		let mut manga = Manga::default();
		details(&page, &mut manga).unwrap();
		manga.viewer
	};
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Reboot</h1></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>South Korea</dd></div></dl></body></html>"#
		),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Super God Gene</h1></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>China</dd></div></dl></body></html>"#
		),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Turning Point</h1><p>Turning Point is a boys' love historical manga.</p><p>터닝포인트</p></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>Japan</dd></div></dl></body></html>"#
		),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Original Title Case</h1></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>Japan</dd></div><div><dt>Original title</dt><dd><span>오곡 리부트</span></dd></div></dl></body></html>"#
		),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Tagged Strip</h1></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>Japan</dd></div></dl><a href="/genre/long-strip">Long Strip</a></body></html>"#
		),
		Viewer::Webtoon
	);
	assert_eq!(
		viewer_for(
			r#"<html><body><header><h1>Call of the Night</h1><p>よふかしのうた</p></header><dl><div><dt>Type</dt><dd>Manga</dd></div><div><dt>Origin</dt><dd>Japan</dd></div></dl><a href="/genre/romance">Romance</a></body></html>"#
		),
		Viewer::RightToLeft
	);
}
