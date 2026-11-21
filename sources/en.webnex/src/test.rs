use super::*;
use crate::models::ReaderPages;
use aidoku::{
	ContentRating, Home, HomeComponentValue, LinkValue, Viewer, alloc::string::ToString,
	imports::html::Html,
};
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_discover_uses_the_requested_headings_and_real_chapters() {
	let c = "<li class='card-item'><a class='card-link' href='/manga/story'><img class='cover-img' src='/cover/story'><h3 class='card-title'>A story</h3></a><span class='card-rank'>1</span><p class='card-meta'><span>Manhwa</span><span>202 readers</span></p><ul class='card-chapters'><li><a href='/read/upload'><span class='card-ch-label'>Ch.12.5</span><span class='card-ch-time'>2h ago</span></a></li></ul></li>";
	let side = "<ol class='side-list'><li><a class='side-row' href='/manga/story'><span class='side-title'>A story</span></a></li></ol>";
	let mut html = format!(
		"<div id='S:8' hidden>{c}</div><section aria-labelledby='popular-heading'><template id='P:8'></template></section><section aria-labelledby='taste-heading'><h2>Your taste</h2><ol>{}</ol></section>",
		c.replace("/manga/story", "/manga/personal")
	);
	for (id, title) in [
		("manhua", "Manhua"),
		("favourites", "Readers’ favourites"),
		("latest", "Latest updates"),
		("manga", "Manga"),
		("trending", "Recently popular"),
		("new", "New in 2026"),
		("manhwa", "Manhwa"),
	] {
		html.push_str(&format!(
			"<section aria-labelledby='{id}-heading'><h2>{title}</h2><a class='see-all' href='/browse?kind={id}&amp;sort=updated'>View all</a><ol>{c}</ol></section>"
		));
	}
	for tab in ["added", "finished"] {
		html.push_str(&format!("<div role='tabpanel' id='x-{tab}'>{side}</div>"));
	}
	let doc = Html::parse_with_url(html, BASE_URL).unwrap();
	let rows = home::layout(&doc, 8000, None).components;
	assert_eq!(
		rows.iter()
			.map(|r| r.title.as_deref().unwrap())
			.collect::<Vec<_>>(),
		[
			"Popular This Week",
			"Your Taste",
			"Readers Favourites",
			"New in 2026",
			"Recently Popular",
			"Latest Updates",
			"Manhwa",
			"Manga",
			"Manhua",
			"Finished",
			"Just Added"
		]
	);
	assert!(
		matches!(&rows[0].value,HomeComponentValue::BigScroller{entries,..} if entries[0].key=="story" && entries[0].description.is_none() && entries[0].tags.as_ref().unwrap()[0]=="Manhwa")
	);
	assert!(
		matches!(&rows[2].value,HomeComponentValue::MangaList{ranking:true,page_size:Some(3),entries,..} if matches!(&entries[0].value,Some(LinkValue::Manga(m)) if m.tags.is_none() && m.key=="story"))
	);
	assert!(
		matches!(&rows[1].value,HomeComponentValue::MangaList{ranking:true,page_size:Some(3),entries,listing:None} if matches!(&entries[0].value,Some(LinkValue::Manga(m)) if m.key=="personal"))
	);
	assert!(matches!(
		&rows[9].value,
		HomeComponentValue::MangaList {
			ranking: true,
			page_size: Some(3),
			..
		}
	));
	assert!(matches!(
		&rows[10].value,
		HomeComponentValue::Scroller { .. }
	));
	assert!(
		matches!(&rows[5].value,HomeComponentValue::MangaChapterList{entries,page_size:Some(5),..} if entries[0].chapter.chapter_number==Some(12.5) && entries[0].chapter.date_uploaded==Some(800) && entries[0].chapter.key=="upload")
	);
	assert!(
		matches!(&rows[5].value,HomeComponentValue::MangaChapterList{listing:Some(l),..} if l.id=="/browse?kind=latest&sort=updated" && l.name=="Latest Updates")
	);
	assert!(
		matches!(&rows[6].value,HomeComponentValue::Scroller{listing:Some(l),..} if l.id=="/browse?kind=manhwa&sort=updated" && l.name=="Manhwa")
	);
	let shown = vec!["new".into(), "taste".into()];
	assert_eq!(
		home::layout(&doc, 0, Some(&shown))
			.components
			.iter()
			.filter_map(|c| c.title.as_deref())
			.collect::<Vec<_>>(),
		["Readers Favourites", "New in 2026"]
	);
	assert!(home::layout(&doc, 0, Some(&[])).components.is_empty());
	// Unpersonalised, "Your taste" is only a loading skeleton and the stream fills in favourites.
	let anonymous = Html::parse_with_url(
		format!(
			"<section aria-label='Your taste'><h2>Your taste</h2><ol>{c}</ol></section><div hidden id='S:3'><section aria-labelledby='taste-heading'><h2>Readers’ favourites</h2><ol>{c}</ol></section></div>"
		),
		BASE_URL,
	)
	.unwrap();
	assert_eq!(
		home::layout(&anonymous, 0, None)
			.components
			.iter()
			.filter_map(|c| c.title.as_deref())
			.collect::<Vec<_>>(),
		["Readers Favourites"]
	);
}

#[aidoku_test]
fn review_preset_controls_and_section_defaults() {
	let filters: serde_json::Value =
		serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	let filters = filters.as_array().unwrap();
	assert!(!filters.iter().any(|f| f["id"] == "tags"));
	let position = |id| filters.iter().position(|f| f["id"] == id).unwrap();
	assert_eq!(position("chapters"), position("minChapters") + 1);
	for (id, first) in [("chapters", "1"), ("released", "this-year")] {
		let f = &filters[position(id)];
		assert_eq!(f["default"], "");
		assert_eq!(f["usesTagStyle"], true);
		assert_eq!(f["ids"][0], first);
		assert_eq!(
			f["options"].as_array().unwrap().len(),
			f["ids"].as_array().unwrap().len()
		);
		assert!(!f["options"].as_array().unwrap().iter().any(|v| v == "Any"));
	}
	let settings: serde_json::Value =
		serde_json::from_str(include_str!("../res/settings.json")).unwrap();
	let sections = &settings[1]["items"][0];
	assert_eq!(sections["key"], "homeSections");
	assert_eq!(sections["default"], sections["values"]);
	assert_eq!(
		sections["values"]
			.as_array()
			.unwrap()
			.iter()
			.filter_map(|v| v.as_str())
			.collect::<Vec<_>>(),
		[
			"popular", "yours", "taste", "new", "trending", "latest", "manhwa", "manga", "manhua",
			"finished", "added"
		]
	);
	use aidoku::imports::defaults::{DefaultValue, defaults_set};
	defaults_set("homeSections", DefaultValue::StringArray(vec![]));
	assert!(Webnex.get_home().unwrap().components.is_empty());
	defaults_set(
		"homeSections",
		DefaultValue::StringArray(
			sections["values"]
				.as_array()
				.unwrap()
				.iter()
				.map(|v| v.as_str().unwrap().into())
				.collect(),
		),
	);
	for key in ["contentRatings", "contentTypes"] {
		defaults_set(key, DefaultValue::StringArray(vec![]));
		assert!(
			Webnex
				.get_search_manga_list(None, 1, vec![])
				.unwrap()
				.entries
				.is_empty()
		);
		assert!(Webnex.get_home().unwrap().components.is_empty());
		let values = if key == "contentRatings" {
			&settings::CONTENT_RATINGS[..2]
		} else {
			&settings::CONTENT_TYPES[..]
		};
		defaults_set(
			key,
			DefaultValue::StringArray(values.iter().map(|v| (*v).into()).collect()),
		);
	}
}

#[aidoku_test]
fn review_details_preserve_prose_emphasis_links_and_credit_order() {
	let doc = Html::parse_with_url("<h1 class='series-title'>Story</h1><p class='series-credits'><a href='?author=A'>A</a><a href='?author=A'>A</a><a href='?author=B'>B</a><a href='?artist=C'>C</a></p><div class='series-desc'><p>[A] * B <strong>bold <i>and italic</i></strong><br>Next <a href='https://example.com'>link</a>.</p><p>Second.</p></div>", BASE_URL).unwrap();
	let details = models::details(&doc, "story").unwrap();
	assert_eq!(details.authors.unwrap(), ["A", "B"]);
	assert_eq!(details.artists.unwrap(), ["C"]);
	let text = details.description.unwrap();
	assert!(
		text.contains("\\[A\\] \\* B")
			&& text.contains("**bold *and italic***")
			&& text.contains("[link](https://example.com/)"),
		"{text}"
	);
	assert!(
		text.contains("  \nNext") && text.ends_with("\n\nSecond\\."),
		"{text}"
	);
	for (badges, tags, viewer) in [
		("<li>Manga</li><li>Japanese</li>", "", Viewer::RightToLeft),
		(
			"<li>Manga</li><li>Japanese</li>",
			"<a class='chip'>Longstrip</a>",
			Viewer::Webtoon,
		),
		("<li>Manga</li><li>Korean</li>", "", Viewer::Webtoon),
		("<li>Manhwa</li>", "", Viewer::Webtoon),
	] {
		let doc = Html::parse(format!(
			"<h1 class='series-title'>Story</h1><ul class='series-badges'>{badges}</ul><div class='detail-list'>{tags}</div>"
		))
		.unwrap();
		assert_eq!(
			models::details(&doc, "story").unwrap().viewer,
			viewer,
			"{badges} {tags}"
		);
	}
	for html in ["", "<div class='series-desc'><p></p></div>"] {
		let doc = Html::parse(format!("<h1 class='series-title'>Story</h1>{html}")).unwrap();
		assert!(
			models::details(&doc, "story")
				.unwrap()
				.description
				.is_none()
		);
	}
}

#[aidoku_test]
fn review_finished_reassembles_nested_titles_and_side_rows() {
	let doc = Html::parse_with_url("<div role='tabpanel' id='x-finished'><ol class='side-list'><li><a class='side-row' href='/manga/first'><template id='P:a'></template></a></li><template id='P:b'></template></ol></div><div id='S:b'><li><a class='side-row' href='/manga/second'><template id='P:c'></template></a></li></div><div id='S:c'><span class='side-title'>Second</span></div><div id='S:a'><span class='side-title'>First</span></div>", BASE_URL).unwrap();
	let rows = home::layout(&doc, 0, None).components;
	let HomeComponentValue::MangaList { entries, .. } = &rows[0].value else {
		panic!()
	};
	let titles: Vec<_> = entries
		.iter()
		.filter_map(|entry| match &entry.value {
			Some(LinkValue::Manga(m)) => Some(m.title.as_str()),
			_ => None,
		})
		.collect();
	assert_eq!(titles, ["First", "Second"]);
}

#[aidoku_test]
fn review_full_type_selection_sends_no_kind() {
	let mut prefs = Catalogue {
		ratings: vec!["safe".into()],
		types: vec!["manga".into(), "manhwa".into(), "manhua".into()],
		hidden_genres: vec![],
	};
	let qs = helpers::search_params(None, 1, vec![], &mut prefs)
		.unwrap()
		.to_string();
	assert!(!qs.contains("kind="), "{qs}");
	prefs.types.pop();
	let qs = helpers::search_params(None, 1, vec![], &mut prefs)
		.unwrap()
		.to_string();
	assert!(
		qs.contains("kind=manga") && qs.contains("kind=manhwa"),
		"{qs}"
	);
}

#[aidoku_test]
fn review_catalogue_defaults_overrides_and_clearing() {
	let mut prefs = Catalogue {
		ratings: vec![],
		types: vec![],
		hidden_genres: vec!["Action".into()],
	};
	assert!(prefs.ceiling().is_none());
	let multi = |id: &str, v: &str| FilterValue::MultiSelect {
		id: id.into(),
		included: vec![v.into()],
		excluded: vec![],
	};
	let qs = helpers::search_params(
		Some("A & B"),
		2,
		vec![
			multi("rating", "erotica"),
			multi("kind", "manhua"),
			multi("genre", "Action"),
		],
		&mut prefs,
	)
	.unwrap()
	.to_string();
	assert!(
		qs.contains("rated=erotica")
			&& qs.contains("rating=erotica")
			&& qs.contains("kind=manhua")
			&& qs.contains("tag=Action")
			&& qs.contains("page=2")
			&& !qs.contains("safe")
	);
	assert!(prefs.hidden_genres.is_empty());
	let prefs = Catalogue::load();
	assert_eq!(prefs.ratings, ["safe", "suggestive"]);
	assert_eq!(prefs.types, ["manga", "manhwa", "manhua"]);
	assert!(!Groups::load().one_per_chapter);
}

#[aidoku_test]
fn review_filters_send_tags_typed_chapters_and_release_presets() {
	let mut prefs = Catalogue::load();
	let select = |id: &str, value: &str| FilterValue::Select {
		id: id.into(),
		value: value.into(),
	};
	let text = |id: &str, value: &str| FilterValue::Text {
		id: id.into(),
		value: value.into(),
	};
	let query = |filters, prefs: &mut Catalogue| {
		helpers::search_params(None, 1, filters, prefs)
			.unwrap()
			.to_string()
	};
	let qs = query(
		vec![
			FilterValue::MultiSelect {
				id: "theme".into(),
				included: vec!["Magic".into()],
				excluded: vec!["Zombies".into()],
			},
			select("genre", "Isekai"),
			select("chapters", "50"),
			text("minChapters", " 37 "),
			select("released", "2010s"),
			FilterValue::Range {
				id: "year".into(),
				from: Some(2012.0),
				to: None,
			},
		],
		&mut prefs,
	);
	for part in [
		"tag=Magic",
		"xtag=Zombies",
		"tag=Isekai",
		"chapters=37",
		"from=2012",
		"to=2019",
	] {
		assert!(qs.contains(part), "{part} in {qs}");
	}
	assert!(!qs.contains("chapters=50") && !qs.contains("genre="));
	let qs = query(
		vec![select("chapters", "1"), select("released", "before-2000")],
		&mut prefs,
	);
	assert!(qs.contains("chapters=1") && qs.contains("to=1999") && !qs.contains("from="));
	for value in ["many", "0", "10001"] {
		assert!(
			helpers::search_params(None, 1, vec![text("minChapters", value)], &mut prefs).is_err()
		);
	}
	let qs = query(vec![select("genre", "Manhua")], &mut prefs);
	assert!(!qs.contains("tag=") && prefs.types == ["manhua"]);
	for n in ["1", "10", "50", "100", "200"] {
		assert!(query(vec![select("chapters", n)], &mut prefs).contains(&format!("chapters={n}")));
	}
	for (preset, expected) in [
		("this-year", "from="),
		("last-5-years", "from="),
		("2010s", "from=2010&to=2019"),
		("2000s", "from=2000&to=2009"),
		("before-2000", "to=1999"),
	] {
		assert!(query(vec![select("released", preset)], &mut prefs).contains(expected));
	}
	let cleared = query(vec![], &mut prefs);
	assert!(
		!cleared.contains("chapters=") && !cleared.contains("from=") && !cleared.contains("to=")
	);
}

#[aidoku_test]
fn review_group_selection_keeps_parts_and_excludes_before_ranking() {
	let row = |part: &str| {
		format!(
			"<li class='ch-row'><a class='ch-link' href='/read/a-{part}'><span class='ch-num'>2</span><span class='ch-title'>{part}</span></a><span class='ch-source'>Alpha</span><div class='ch-sources-menu'><a class='ch-sources-item' href='/read/b-{part}'><span class='ch-sources-name'>Beta</span><span class='ch-sources-time'>2mo ago</span></a></div></li>"
		)
	};
	let doc = Html::parse(format!("<ul>{}{}</ul>", row("first"), row("second"))).unwrap();
	let mut groups = Groups {
		hidden: vec![],
		preferred: vec!["beta".into()],
		one_per_chapter: false,
	};
	assert_eq!(
		models::select_chapters(models::uploads(&doc, &groups, 6000000).unwrap(), &groups).len(),
		4
	);
	groups.one_per_chapter = true;
	let chapters =
		models::select_chapters(models::uploads(&doc, &groups, 6000000).unwrap(), &groups);
	assert_eq!(
		chapters.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(),
		["b-first", "b-second"]
	);
	assert_eq!(chapters[0].date_uploaded, Some(816000));
	groups.hidden.push("beta".into());
	assert_eq!(
		models::select_chapters(models::uploads(&doc, &groups, 0).unwrap(), &groups)[0].key,
		"a-first"
	);
	groups.hidden.clear();
	groups.preferred.clear();
	let row = |key| {
		format!(
			"<li class='ch-row'><a class='ch-link' href='/read/{key}'><span class='ch-num'>1</span></a><span class='ch-source'>{key}</span></li>"
		)
	};
	for rows in [
		format!("{}{}", row("z"), row("a")),
		format!("{}{}", row("a"), row("z")),
	] {
		let doc = Html::parse(format!("<ol>{rows}</ol>")).unwrap();
		let chosen = models::select_chapters(models::uploads(&doc, &groups, 0).unwrap(), &groups);
		assert_eq!(chosen.len(), 1);
		assert_eq!(chosen[0].key, "a");
	}
}

#[aidoku_test]
fn review_streamed_chapters_keep_links_pagination_and_official_credits() {
	let doc = Html::parse(concat!(
		"<section id='chapters'><ol class='ch-list'><li class='ch-row'>",
		"<template id='P:5'></template><template id='P:6'></template></li>",
		"<template id='P:8'></template></ol><template id='P:9'></template><template id='P:a'></template></section>",
		"<div hidden id='S:5'><a class='ch-link' href='/read/official'><span class='ch-num'>",
		"<span aria-hidden='true'>Ch. </span><span class='visually-hidden'>Chapter </span>21</span></a></div>",
		"<div hidden id='S:6'><div class='ch-meta'><button class='ch-source-trigger'>",
		"<span class='ch-source-name'>WEBTOON</span><span class='verified-mark' aria-label='Official translation'></span></button>",
		"<div class='ch-sources-menu'><a class='ch-sources-item' href='/read/alternate'>",
		"<span class='ch-sources-name'>Fan Group</span></a></div></div></div>",
		"<div hidden id='S:8'><li class='ch-row'><a class='ch-link' href='/read/next'>",
		"<span class='ch-num'>20</span></a><time class='ch-time' datetime='2026-09-19T23:40:35.000Z'></time><span class='ch-source'><span class='ch-credit-group'>Other Group</span></span></li></div>",
		"<div hidden id='S:9'><ol class='pagination-pages'><li><a href='?page=3#chapters'>3</a></li></ol></div>",
		"<div hidden id='S:a'></div>"
	)).unwrap();
	let doc = rsc::chapter_list(&doc).unwrap();
	assert!(doc.select_first("a[href='?page=3#chapters']").is_some());
	let mut groups = Groups {
		hidden: vec![],
		preferred: vec![],
		one_per_chapter: false,
	};
	let chapters = models::select_chapters(models::uploads(&doc, &groups, 0).unwrap(), &groups);
	assert_eq!(chapters.len(), 3);
	let official = chapters
		.iter()
		.find(|chapter| chapter.key == "official")
		.unwrap();
	assert_eq!(official.chapter_number, Some(21.0));
	assert_eq!(official.scanlators.as_deref().unwrap(), ["WEBTOON ✓"]);
	let alternate = chapters
		.iter()
		.find(|chapter| chapter.key == "alternate")
		.unwrap();
	assert_eq!(alternate.scanlators.as_deref().unwrap(), ["Fan Group"]);
	let next = chapters
		.iter()
		.find(|chapter| chapter.key == "next")
		.unwrap();
	assert_eq!(next.date_uploaded, Some(1789861235));

	groups.hidden.push("webtoon".into());
	let chapters = models::select_chapters(models::uploads(&doc, &groups, 0).unwrap(), &groups);
	assert_eq!(
		chapters
			.iter()
			.map(|chapter| chapter.key.as_str())
			.collect::<Vec<_>>(),
		["alternate", "next"]
	);
}

#[aidoku_test]
fn review_key_only_links_and_complete_page_lists() {
	assert_eq!(
		Webnex
			.handle_deep_link(format!("{BASE_URL}/manga/story?source=group"))
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "story".into()
		})
	);
	assert_eq!(
		Webnex
			.handle_deep_link("https://www.webnex.cc/manga/story".into())
			.unwrap(),
		Some(DeepLinkResult::Manga {
			key: "story".into()
		})
	);
	let props = concat!(
		r#"1:{"chapter":{},"series":{"href":"/manga/story"}}"#,
		"\n",
		r#"2:{"chapter":{},"series":{"href":"/manga/story"},"pagesUrl":"/read/canonical/pages"}"#
	);
	let doc = Html::parse(format!(
		"<script>self.__next_f.push([1,{}])</script>",
		serde_json::to_string(props).unwrap()
	))
	.unwrap();
	let decoded: ReaderProps = rsc::extract(&doc, &["chapter", "series", "pagesUrl"]).unwrap();
	assert_eq!(decoded.series.href, "/manga/story");
	assert_eq!(decoded.pages_url, "/read/canonical/pages");
	for (script, cookie) in [
		(
			r#"document.cookie=["wnx_reader_v2=17.ab","Max-Age=21600","Path=/read/x"].join("; ")"#,
			Some("wnx_reader_v2=17.ab"),
		),
		(
			r#"document.cookie="wnx_series=cd; Path=/";location.reload()"#,
			Some("wnx_series=cd"),
		),
		(r#"var label="wnx_reader_v2=17.ab""#, None),
	] {
		let doc = Html::parse(format!("<script>{script}</script>")).unwrap();
		assert_eq!(helpers::script_cookie(&doc).as_deref(), cookie, "{script}");
	}
	let result = |urls: &[&str]| {
		Vec::<Page>::try_from(ReaderPages {
			pages: urls.iter().map(|s| (*s).into()).collect(),
			message: None,
		})
	};
	assert!(result(&[]).is_err());
	assert!(result(&["/image/1", ""]).is_err());
	assert_eq!(
		result(&["/image/1", "https://images.webnex.cc/p2"])
			.unwrap()
			.len(),
		2
	);
}

#[aidoku_test]
fn live_pasted_links_resolve_or_fall_back_to_search() {
	let pasted = |url: String| {
		Webnex
			.get_search_manga_list(Some(url), 1, vec![])
			.unwrap()
			.entries
	};
	let found = pasted(format!("{BASE_URL}/manga/hell-login-gocnsptt2v"));
	assert_eq!(found.len(), 1);
	assert_eq!(found[0].key, "hell-login-gocnsptt2v");
	assert!(found[0].description.is_none());
	// An unknown series is searched for as text rather than failing the search.
	pasted(format!("{BASE_URL}/manga/no-such-series-zz9"));
}

#[aidoku_test]
fn live_search_pages_and_series_updates() {
	let first = Webnex.get_search_manga_list(None, 1, vec![]).unwrap();
	let next = Webnex.get_search_manga_list(None, 2, vec![]).unwrap();
	assert!(first.has_next_page);
	assert_ne!(first.entries[0].key, next.entries[0].key);
	let key = "hell-login-gocnsptt2v";
	for (details, chapters) in [(true, false), (false, true), (true, true)] {
		let m = Webnex
			.get_manga_update(
				Manga {
					key: key.into(),
					..Default::default()
				},
				details,
				chapters,
			)
			.unwrap();
		if details {
			assert_eq!(m.title, "Hell Login");
			assert_eq!(m.content_rating, ContentRating::Suggestive);
			assert_eq!(m.viewer, Viewer::Webtoon);
		}
		if chapters {
			assert!(
				m.chapters
					.unwrap()
					.iter()
					.any(|c| c.chapter_number.is_some() && c.date_uploaded.is_some())
			);
		}
	}
}

#[aidoku_test]
fn live_discover_and_chapter_link() {
	let rows = Webnex.get_home().unwrap().components;
	assert!(
		rows.iter()
			.any(|c| matches!(c.value, HomeComponentValue::BigScroller { .. }))
	);
	let titles: Vec<_> = rows.iter().filter_map(|c| c.title.as_deref()).collect();
	let position = |title| titles.iter().position(|t| *t == title).unwrap();
	assert!(
		position("Recently Popular") < position("Manhwa")
			&& position("Manhwa") < position("Manga")
			&& position("Manga") < position("Manhua"),
		"{titles:?}"
	);
	assert_eq!(
		Webnex
			.handle_deep_link(format!("{BASE_URL}/read/gocnsptt2v-wii518ji"))
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			key: "gocnsptt2v-wii518ji".into(),
			manga_key: "hell-login-gocnsptt2v".into()
		})
	);
}

#[aidoku_test]
fn live_chapters_include_the_last_page() {
	let manga = Webnex
		.get_manga_update(
			Manga {
				key: "revenge-of-the-iron-blooded-sword-hound-47ankmbtbo".into(),
				..Default::default()
			},
			false,
			true,
		)
		.unwrap();
	let chapters = manga.chapters.unwrap();
	assert!(chapters.iter().any(|c| c.chapter_number == Some(1.0)));
	assert!(
		chapters
			.iter()
			.any(|c| c.chapter_number.is_some_and(|n| n >= 150.0))
	);
}

#[aidoku_test]
fn live_exact_ratings_override_the_default() {
	for (rating, expected) in [
		("safe", ContentRating::Safe),
		("suggestive", ContentRating::Suggestive),
		("erotica", ContentRating::NSFW),
		("pornographic", ContentRating::NSFW),
	] {
		let filter = FilterValue::MultiSelect {
			id: "rating".into(),
			included: vec![rating.into()],
			excluded: vec![],
		};
		let result = Webnex.get_search_manga_list(None, 1, vec![filter]).unwrap();
		let title = result.entries.into_iter().next().unwrap();
		let details = Webnex.get_manga_update(title, true, false).unwrap();
		assert_eq!(
			details.content_rating, expected,
			"{rating}: {}",
			details.title
		);
	}
}

#[aidoku_test]
fn live_typed_minimum_and_release_presets_narrow_the_catalogue() {
	let keys = |filters| {
		Webnex
			.get_search_manga_list(None, 1, filters)
			.unwrap()
			.entries
			.into_iter()
			.map(|m| m.key)
			.collect::<Vec<_>>()
	};
	let all = keys(vec![]);
	for filter in [
		FilterValue::Text {
			id: "minChapters".into(),
			value: "300".into(),
		},
		FilterValue::Select {
			id: "released".into(),
			value: "before-2000".into(),
		},
	] {
		let narrowed = keys(vec![filter]);
		assert!(!narrowed.is_empty());
		assert_ne!(narrowed, all);
	}
}

#[aidoku_test]
fn live_sorts_and_tag_exclusions_match_details() {
	for index in 0..6 {
		let result = Webnex
			.get_search_manga_list(
				None,
				1,
				vec![FilterValue::Sort {
					id: "sort".into(),
					index,
					ascending: false,
				}],
			)
			.unwrap();
		assert!(!result.entries.is_empty(), "sort {index}");
		assert!(
			result
				.entries
				.iter()
				.all(|m| !m.key.is_empty() && !m.title.is_empty()),
			"sort {index}"
		);
	}
	for (id, tag) in [
		("genre", "Action"),
		("format", "Long Strip"),
		("theme", "Magic"),
	] {
		for exclude in [false, true] {
			let result = Webnex
				.get_search_manga_list(
					None,
					1,
					vec![FilterValue::MultiSelect {
						id: id.into(),
						included: if exclude { vec![] } else { vec![tag.into()] },
						excluded: if exclude { vec![tag.into()] } else { vec![] },
					}],
				)
				.unwrap();
			assert!(!result.entries.is_empty(), "{id} {tag} excluded={exclude}");
			let manga = Webnex
				.get_manga_update(result.entries.into_iter().next().unwrap(), true, false)
				.unwrap();
			assert_eq!(
				manga.tags.unwrap().iter().any(|t| t == tag),
				!exclude,
				"{id} {tag} excluded={exclude}"
			);
		}
	}
	let found = Webnex
		.get_search_manga_list(
			Some("Hell Login".into()),
			1,
			vec![FilterValue::Select {
				id: "chapters".into(),
				value: "1".into(),
			}],
		)
		.unwrap();
	assert!(
		found
			.entries
			.iter()
			.any(|m| m.key == "hell-login-gocnsptt2v")
	);
	let impossible = Webnex
		.get_search_manga_list(
			Some("Hell Login".into()),
			1,
			vec![FilterValue::Text {
				id: "minChapters".into(),
				value: "10000".into(),
			}],
		)
		.unwrap();
	assert!(impossible.entries.is_empty());
}

#[aidoku_test]
fn live_page_list_loads_from_the_chapter_cookie() {
	let chapter = Chapter {
		key: "gocnsptt2v-wii518ji".into(),
		..Default::default()
	};
	let pages = Webnex.get_page_list(Manga::default(), chapter).unwrap();
	assert!(pages.len() > 1);
	assert!(pages.iter().all(
		|p| matches!(&p.content, aidoku::PageContent::Url(url, _) if url.starts_with("https://"))
	));
}

#[aidoku_test]
fn live_long_strips_labelled_manga_open_as_webtoons() {
	for (key, viewer) in [
		(
			"having-turned-back-time-i-want-to-live-a-different-life-uwo9f4cgvs",
			Viewer::Webtoon,
		),
		(
			"after-being-called-a-hero-the-unrivaled-man-starts-a-family-qy8l11uog6",
			Viewer::RightToLeft,
		),
	] {
		let manga = Manga {
			key: key.into(),
			..Default::default()
		};
		assert_eq!(
			Webnex.get_manga_update(manga, true, false).unwrap().viewer,
			viewer,
			"{key}"
		);
	}
}

#[aidoku_test]
fn review_unknown_listing_links_fail_instead_of_widening() {
	let listing = Listing {
		id: "/browse?shelf=staff&sort=updated".into(),
		name: "Staff Picks".into(),
		..Default::default()
	};
	assert!(Webnex.get_manga_list(listing, 1).is_err());
}

#[aidoku_test]
fn live_view_all_lists_continue_their_discover_rows() {
	let mut checked = 0;
	for row in Webnex.get_home().unwrap().components {
		let (keys, listing): (Vec<_>, _) = match row.value {
			HomeComponentValue::Scroller {
				entries,
				listing: Some(listing),
			} => (
				entries
					.into_iter()
					.filter_map(|e| match e.value {
						Some(LinkValue::Manga(m)) => Some(m.key),
						_ => None,
					})
					.collect(),
				listing,
			),
			HomeComponentValue::MangaChapterList {
				entries,
				listing: Some(listing),
				..
			} => (entries.into_iter().map(|e| e.manga.key).collect(), listing),
			_ => continue,
		};
		assert!(!keys.is_empty());
		// The site's own View all page, unfiltered, is what the listing must reproduce.
		let site = helpers::get_html(&format!("{BASE_URL}{}", listing.id), None)
			.unwrap_or_else(|e| panic!("{}: {e:?}", listing.id));
		let section = site.select_first("section.browse").unwrap();
		let html = rsc::expand(&section, &mut rsc::chunks(&site)).unwrap();
		let expected: Vec<_> = Html::parse_with_url(html, BASE_URL)
			.unwrap()
			.select("li.card-item")
			.unwrap()
			.filter_map(|el| helpers::card(&el))
			.map(|m| m.key)
			.take(6)
			.collect();
		let name = listing.name.clone();
		let list = Webnex.get_manga_list(listing, 1).unwrap();
		assert!(list.has_next_page, "{name}");
		let listed: Vec<_> = list.entries.into_iter().map(|m| m.key).collect();
		assert!(
			expected.iter().filter(|k| listed.contains(k)).count() >= 4,
			"{name}: {expected:?} mostly missing from {listed:?}"
		);
		checked += 1;
	}
	assert!(checked >= 6, "only {checked} rows had a listing");
}

#[aidoku_test]
fn live_skipped_ratings_keep_the_rows_browse_can_match() {
	use aidoku::imports::defaults::{DefaultValue, defaults_set};
	defaults_set(
		"contentRatings",
		DefaultValue::StringArray(vec![
			"safe".into(),
			"suggestive".into(),
			"pornographic".into(),
		]),
	);
	let rows = Webnex.get_home().unwrap().components;
	let year = helpers::current_year();
	assert_eq!(
		rows.iter()
			.filter_map(|r| r.title.clone())
			.collect::<Vec<_>>(),
		[
			"Popular This Week".into(),
			format!("New in {year}"),
			"Latest Updates".into(),
			"Manhwa".into(),
			"Manga".into(),
			"Manhua".into()
		]
	);
	assert!(
		matches!(&rows[0].value, HomeComponentValue::BigScroller { entries, .. } if !entries.is_empty())
	);
	assert!(rows[1..].iter().all(|r| matches!(&r.value,
		HomeComponentValue::Scroller { entries, listing: Some(l) } if !entries.is_empty() && l.id.starts_with("/browse?"))));
	defaults_set(
		"homeSections",
		DefaultValue::StringArray(vec!["taste".into(), "manga".into()]),
	);
	assert_eq!(
		Webnex
			.get_home()
			.unwrap()
			.components
			.iter()
			.filter_map(|r| r.title.as_deref())
			.collect::<Vec<_>>(),
		["Manga"]
	);
}
