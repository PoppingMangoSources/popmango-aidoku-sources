use crate::{
	XComic,
	graphql::{
		BrowseParams, EXTENDED_PAGE_SIZE, PAGE_SIZE, browse_request, latest_uploads_request,
		parse_browse, parse_latest_uploads,
	},
	helpers::{chapter_from_data, manga_from_data, type_label},
	models::{ComicData, LatestEntry},
	settings,
};
use aidoku::{
	BaseUrlProvider, Home, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	ListingKind, Manga, MangaWithChapter, Result,
	alloc::{String, Vec, vec},
	imports::net::{Request, RequestError, Response},
};

// Lets a section header open the matching `source.json` listing.
fn listing(id: &str, name: &str) -> Option<Listing> {
	Some(Listing {
		id: id.into(),
		name: name.into(),
		kind: ListingKind::Default,
	})
}

// Home sections only render covers, so coverless entries are dropped.
fn visible(comic: ComicData, base_url: &str) -> Option<Manga> {
	let manga = manga_from_data(comic, base_url);
	manga.cover.is_some().then_some(manga)
}

/// Entries for the big scroller, which renders the description itself.
fn manga_section(comics: Vec<ComicData>, base_url: &str, limit: usize) -> Vec<Manga> {
	comics
		.into_iter()
		.filter_map(|comic| visible(comic, base_url))
		.take(limit)
		.collect()
}

/// Links with an explicit subtitle, which `From<Manga>` would otherwise leave
/// empty for these compact browse results.
fn links(
	comics: Vec<ComicData>,
	base_url: &str,
	limit: usize,
	subtitle: impl Fn(&ComicData) -> Option<String>,
) -> Vec<Link> {
	comics
		.into_iter()
		.filter_map(|comic| {
			let subtitle = subtitle(&comic);
			let mut link = Link::from(visible(comic, base_url)?);
			link.subtitle = subtitle;
			Some(link)
		})
		.take(limit)
		.collect()
}

/// Pairs each upload with its chapter so the app can show a relative timestamp.
fn chapter_entries(items: Vec<LatestEntry>, base_url: &str, limit: usize) -> Vec<MangaWithChapter> {
	items
		.into_iter()
		.filter_map(|(comic, chapter)| {
			let language = comic
				.translated_language
				.as_deref()
				.and_then(settings::normalize_language);
			let chapter = chapter_from_data(chapter?, base_url, language.as_deref(), true)?;
			Some(MangaWithChapter {
				manga: visible(comic, base_url)?,
				chapter,
			})
		})
		.take(limit)
		.collect()
}

impl Home for XComic {
	fn get_home(&self) -> Result<HomeLayout> {
		let base_url = self.get_base_url()?;
		let mut top_rated_params = BrowseParams::new("field_score", 1)?;
		// The only section that shows more than a cover and a title.
		top_rated_params.detailed = true;
		let most_viewed_params = BrowseParams::new("views_d030", 1)?;
		let latest_params = BrowseParams::new("field_update", 1)?;
		let mut recently_added_params = BrowseParams::new("field_create", 1)?;
		recently_added_params.size = EXTENDED_PAGE_SIZE;
		let most_chapters_params = BrowseParams::new("field_chapter", 1)?;
		let responses: [core::result::Result<Response, RequestError>; 5] = Request::send_all([
			browse_request(&base_url, &top_rated_params)?,
			browse_request(&base_url, &most_viewed_params)?,
			latest_uploads_request(&base_url, None)?,
			browse_request(&base_url, &recently_added_params)?,
			browse_request(&base_url, &most_chapters_params)?,
		])
		.try_into()
		.expect("requests vec length should be 5");
		let [
			top_rated,
			most_viewed,
			latest,
			recently_added,
			most_chapters,
		] = responses;

		let page = PAGE_SIZE as usize;
		let top_rated = manga_section(
			parse_browse(top_rated?, &top_rated_params)?.0,
			&base_url,
			10,
		);
		let most_viewed = links(
			parse_browse(most_viewed?, &most_viewed_params)?.0,
			&base_url,
			usize::MAX,
			|_| None,
		);
		let latest = chapter_entries(
			parse_latest_uploads(latest?, &latest_params)?.0,
			&base_url,
			page,
		);
		let recently_added = links(
			parse_browse(recently_added?, &recently_added_params)?.0,
			&base_url,
			page,
			|_| None,
		);
		let most_chapters = links(
			parse_browse(most_chapters?, &most_chapters_params)?.0,
			&base_url,
			usize::MAX,
			type_label,
		);

		Ok(HomeLayout {
			components: vec![
				HomeComponent {
					title: Some("Top Rated".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries: top_rated,
						auto_scroll_interval: Some(6.0),
					},
				},
				HomeComponent {
					title: Some("Most Viewed (30 Days)".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries: most_viewed,
						listing: listing("views_d030", "Most Viewed (30 Days)"),
					},
				},
				HomeComponent {
					title: Some("Latest Update".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: Some(10),
						entries: latest,
						listing: listing("field_update", "Latest Update"),
					},
				},
				HomeComponent {
					title: Some("Recently Added".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries: recently_added,
						listing: listing("field_create", "Recently Added"),
					},
				},
				HomeComponent {
					title: Some("Most Chapters".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries: most_chapters,
						listing: listing("field_chapter", "Most Chapters"),
					},
				},
			],
		})
	}
}
