use super::*;
use aidoku::{
	ContentRating, HomeComponentValue, MangaStatus, PageContent, Viewer, imports::html::Html,
};
use aidoku_test::aidoku_test;
#[aidoku_test]
fn review_current_details_keep_synopsis_and_independent_credits() {
	let html = Html::parse(r#"<main><h1> Title </h1><div class='col-lg-3'><li><strong>Autor:</strong> First,Second &amp; First</li><li><strong>Artista:</strong> Second</li><li><strong>Estado:</strong> En emisión</li><li><strong>Tipo:</strong> BL Manhwa</li></div><div id='tab-sinopsis'><div id='resume-content'>A "quoted" name.<br><br>Another paragraph.</div><span class='em-badge'>Smut</span></div></main>"#).unwrap();
	let details = helpers::details(&html, "/series/title".into()).unwrap();
	assert_eq!(details.title, "Title");
	assert_eq!(details.authors, Some(vec!["First".into(), "Second".into()]));
	assert_eq!(details.artists, Some(vec!["Second".into()]));
	assert_eq!(details.status, MangaStatus::Ongoing);
	assert_eq!(details.viewer, Viewer::Webtoon);
	assert_eq!(details.content_rating, ContentRating::NSFW);
	let description = details.description.unwrap();
	assert!(description.contains('\n') && description.contains("Another paragraph."));
	assert!(!description.contains("\\r") && !description.contains("\\\""));
}
#[aidoku_test]
fn review_current_reader_keeps_original_images_and_novel_paragraphs() {
	let html=Html::parse("<div id='em-reader-pages'><img class='em-reader-img' src='data:placeholder' data-src='https://example.com/1.webp'><img class='em-reader-img' src='https://example.com/2.webp'></div>").unwrap();
	let pages = helpers::pages(&html).unwrap();
	assert_eq!(pages.len(), 2);
	assert!(
		matches!(&pages[0].content,PageContent::Url(url,_)if url=="https://example.com/1.webp")
	);
	assert!(
		helpers::pages(
			&Html::parse("<div id='em-reader-pages'><img class='em-reader-img' src=' '></div>")
				.unwrap()
		)
		.is_err()
	);
	let html=Html::parse("<div id='em-reader-pages'><article class='novel-content'><p><b>First</b></p><p><i>Second</i></p></article></div>").unwrap();
	let pages = helpers::pages(&html).unwrap();
	assert_eq!(pages[0].content, PageContent::text("**First**\n\n*Second*"));
}
#[aidoku_test]
fn review_current_home_keeps_chapter_number_and_both_date_forms() {
	let html=Html::parse("<main><section><h2>Últimas Actualizaciones</h2><div><a class='em-update-card' href='/series/title'><img alt='Title' src='cover.webp'></a><div><a class='em-chapter-pill' href='/series/title/capitulo-6-part-2'>Cap. 6</a><time datetime='2026-11-11T02:16:14-03:00'></time></div></div></section></main>").unwrap();
	let home = home::parse_home(&html).unwrap();
	let HomeComponentValue::MangaChapterList {
		entries, page_size, ..
	} = &home.components[0].value
	else {
		panic!("Expected chapter rows")
	};
	assert_eq!(*page_size, Some(5));
	assert_eq!(entries[0].chapter.chapter_number, Some(6.));
	assert_eq!(entries[0].chapter.date_uploaded, Some(1794374174));
	let html = Html::parse("<div><span class='em-time-ago'>hace 4 horas</span></div>").unwrap();
	let expected = aidoku::imports::std::current_date() - 14400;
	assert!((helpers::date(&html.select_first("div").unwrap()).unwrap() - expected).abs() <= 1);
}
#[aidoku_test]
fn review_current_filters_and_deep_link_suffixes() {
	let filters: serde_json::Value =
		serde_json::from_str(include_str!("../res/filters.json")).unwrap();
	assert_eq!(filters.as_array().unwrap().len(), 3);
	assert_eq!(filters[2]["id"], "genre");
	assert_eq!(
		EternalMangas::new()
			.handle_deep_link(
				"https://eternalmangas.org/series/title/capitulo-12-part-2/?ref=home#reader".into()
			)
			.unwrap(),
		Some(DeepLinkResult::Chapter {
			manga_key: "/series/title".into(),
			key: "/series/title/capitulo-12-part-2".into()
		})
	);
}
#[aidoku_test]
fn live_current_catalogue_and_reported_titles_load() {
	let source = EternalMangas::new();
	let first = source.get_search_manga_list(None, 1, Vec::new()).unwrap();
	let second = source.get_search_manga_list(None, 2, Vec::new()).unwrap();
	assert!(first.has_next_page && !first.entries.is_empty());
	assert_ne!(first.entries[0].key, second.entries[0].key);
	for slug in [
		"es-un-milagro-pero-es-demasiado",
		"cario-por-qu-no-podemos-divorciarnos",
	] {
		let key = format!("/series/{slug}");
		let details = source
			.get_manga_update(
				Manga {
					key: key.clone(),
					..Default::default()
				},
				true,
				true,
			)
			.unwrap();
		assert!(!details.title.is_empty());
		let description = details.description.unwrap();
		assert!(!description.contains("\\r") && !description.contains("\\\""));
		let chapters = details.chapters.unwrap();
		assert!(chapters.len() >= 12);
		let pages = source
			.get_page_list(
				Manga {
					key: key.clone(),
					..Default::default()
				},
				chapters.into_iter().next().unwrap(),
			)
			.unwrap();
		assert!(!pages.is_empty());
		let pasted = source
			.get_search_manga_list(Some(format!("{URL}{key}")), 1, Vec::new())
			.unwrap();
		assert_eq!(pasted.entries[0].key, key);
		assert!(pasted.entries[0].description.is_none());
	}
}
#[aidoku_test]
fn live_current_home_and_legacy_library_keys_load() {
	let source = EternalMangas::new();
	let home = source.get_home().unwrap();
	assert!(home.components.iter().any(|c|matches!(&c.value,HomeComponentValue::MangaChapterList{entries,..}if !entries.is_empty()&&entries.iter().all(|entry|entry.chapter.chapter_number.is_some()&&entry.chapter.date_uploaded.is_some()))));
	let legacy = source
		.get_manga_update(
			Manga {
				key: "1035".into(),
				..Default::default()
			},
			true,
			true,
		)
		.unwrap();
	assert_eq!(legacy.key, "1035");
	assert!(legacy.chapters.unwrap().len() >= 109);
}
