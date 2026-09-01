use crate::{
	XComic,
	graphql::{
		BrowseParams, EXTENDED_PAGE_SIZE, PAGE_SIZE, browse_request, latest_uploads_request,
		parse_browse, parse_latest_uploads,
	},
	helpers::manga_from_data,
	models::ComicData,
};
use aidoku::{
	BaseUrlProvider, Home, HomeComponent, HomeComponentValue, HomeLayout, Link, Listing,
	ListingKind, Manga, Result,
	alloc::{Vec, vec},
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
fn section<T: From<Manga>>(comics: Vec<ComicData>, base_url: &str, limit: usize) -> Vec<T> {
	comics
		.into_iter()
		.map(|comic| manga_from_data(comic, base_url, false))
		.filter(|manga| manga.cover.is_some())
		.take(limit)
		.map(T::from)
		.collect()
}

impl Home for XComic {
	fn get_home(&self) -> Result<HomeLayout> {
		let base_url = self.get_base_url()?;
		let top_rated_params = BrowseParams::new("field_score", 1)?;
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
		let top_rated: Vec<Manga> = section(
			parse_browse(top_rated?, &top_rated_params)?.0,
			&base_url,
			10,
		);
		let most_viewed: Vec<Link> = section(
			parse_browse(most_viewed?, &most_viewed_params)?.0,
			&base_url,
			usize::MAX,
		);
		let latest: Vec<Link> = section(
			parse_latest_uploads(latest?, &latest_params)?.0,
			&base_url,
			page,
		);
		let recently_added: Vec<Link> = section(
			parse_browse(recently_added?, &recently_added_params)?.0,
			&base_url,
			page,
		);
		let most_chapters: Vec<Link> = section(
			parse_browse(most_chapters?, &most_chapters_params)?.0,
			&base_url,
			usize::MAX,
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
					value: HomeComponentValue::Scroller {
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
