#![no_std]
extern crate alloc;
mod models;

use aidoku::{
	Chapter, ContentRating, DeepLinkHandler, DeepLinkResult, FilterValue, Home, HomeComponent,
	HomeComponentValue, HomeLayout, ImageRequestProvider, Link, Listing, ListingProvider, Manga,
	MangaPageResult, MangaStatus, MangaWithChapter, Page, PageContent, PageContext, Result, Source,
	alloc::{String, Vec, string::ToString, vec},
	imports::{defaults::defaults_get, net::Request},
	prelude::*,
};
use models::*;

const DOMAIN: &str = "https://xcomic.me";
const API_URL: &str = "https://xcomic.me/query/";
const PAGE_SIZE: i32 = 36;
const CHAPTER_PAGE_SIZE: i32 = 1000;

/// Sort ids matching the order of the `sort` filter options in filters.json.
const SORT_IDS: &[&str] = &[
	"field_score",
	"field_update",
	"field_create",
	"field_name_asc",
	"field_name_desc",
	"field_chapter",
	"field_follow",
	"field_review",
	"field_comment",
];

const DEFAULT_TYPES: &[&str] = &["manga", "manhwa", "manhua"];
const DEFAULT_RATINGS: &[&str] = &["safe", "suggestive", "erotica"];

fn content_types() -> Vec<String> {
	defaults_get::<Vec<String>>("contentTypes")
		.filter(|list| !list.is_empty())
		.unwrap_or_else(|| DEFAULT_TYPES.iter().map(|s| s.to_string()).collect())
}

fn content_ratings() -> Vec<String> {
	defaults_get::<Vec<String>>("contentRatings")
		.filter(|list| !list.is_empty())
		.unwrap_or_else(|| DEFAULT_RATINGS.iter().map(|s| s.to_string()).collect())
}

const ADULT_GENRES: &[&str] = &["adult", "hentai", "pornographic", "smut"];
const MATURE_GENRES: &[&str] = &["ecchi", "erotica", "mature", "yaoi", "yuri"];

const BROWSE_QUERY: &str = r#"
query get_comic_browse_items($select: Comic_Browse_Select) {
  get_comic_browse_items(select: $select) {
    data {
      id name urlPath urlCover
      type contentRating genres sfw_result
      summary { html }
      chapterNodes_last(amount: 1) { data { serial chaNum } }
    }
  }
}
"#;

const LATEST_UPDATES_QUERY: &str = r#"
query get_comic_latestUploads($select: Comic_LatestUploads_Select) {
  get_comic_latestUploads(select: $select) {
    before
    items {
      comic { data { id name urlPath urlCover translatedLanguage type contentRating genres sfw_result } }
      chapters(amount: 1) { data { id serial chaNum urlPath dateCreate dateModify datePublic } }
    }
  }
}
"#;

const COMIC_QUERY: &str = r#"
query get_comicNode($id: ID!) {
  get_comicNode(id: $id) {
    data {
      id name altNames
      originalLanguage translatedLanguage
      originalStatus originalPubFrom { y m d }
      originalPubTill { y m d }
      uploadStatus
      type demographics contentRating genres tags
      authorNodes { data { name } }
      artistNodes { data { name } }
      tagNodes { data { name } }
      summary { html }
      urlPath urlCover
      sfw_result score_val chaps_normal
    }
  }
}
"#;

const CHAPTERS_QUERY: &str = r#"
query get_comic_chapterList_uniqList($select: Select_Comic_ChapterList_UniqList) {
  get_comic_chapterList_uniqList(select: $select) {
    paging { pages }
    items {
      data {
        id dbStatus serial chaNum dname title urlPath
        dateCreate dateModify datePublic srcName
        profileNodes { data { name } }
        userNode { data { name } }
        groupNodes { data { name } }
      }
    }
  }
}
"#;

const CHAPTER_PAGES_QUERY: &str = r#"
query get_chapterNode($id: ID!) {
  get_chapterNode(id: $id) { data { imageUrls } }
}
"#;

fn abs_url(url: &str) -> String {
	let trimmed = url.trim();
	if trimmed.is_empty() {
		String::new()
	} else if trimmed.starts_with("http://") || trimmed.starts_with("https://") {
		trimmed.to_string()
	} else if let Some(rest) = trimmed.strip_prefix("//") {
		format!("https://{rest}")
	} else if trimmed.starts_with('/') {
		format!("{DOMAIN}{trimmed}")
	} else {
		format!("{DOMAIN}/{trimmed}")
	}
}

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Strips HTML tags, turning block breaks into newlines.
fn strip_html(html: &str) -> String {
	let normalized = html
		.replace("<br>", "\n")
		.replace("<br/>", "\n")
		.replace("<br />", "\n")
		.replace("</p>", "\n")
		.replace("</div>", "\n")
		.replace("</li>", "\n");
	let mut out = String::with_capacity(normalized.len());
	let mut in_tag = false;
	for ch in normalized.chars() {
		match ch {
			'<' => in_tag = true,
			'>' => in_tag = false,
			_ if !in_tag => out.push(ch),
			_ => {}
		}
	}
	out.lines()
		.map(str::trim)
		.filter(|line| !line.is_empty())
		.collect::<Vec<_>>()
		.join("\n")
}

fn title_case(value: &str) -> String {
	let spaced = value.replace('_', " ");
	let mut out = String::with_capacity(spaced.len());
	let mut capitalize = true;
	for ch in spaced.chars() {
		if capitalize && ch.is_alphabetic() {
			out.extend(ch.to_uppercase());
			capitalize = false;
		} else {
			out.push(ch);
			if ch == ' ' {
				capitalize = true;
			}
		}
	}
	out
}

fn node_names(nodes: Option<&Vec<NamedNode>>) -> Vec<String> {
	nodes
		.map(|list| {
			list.iter()
				.filter_map(|node| node.data.as_ref().and_then(|d| d.name.as_deref()))
				.map(clean)
				.filter(|name| !name.is_empty())
				.collect()
		})
		.unwrap_or_default()
}

fn content_rating_for(rating: Option<&str>, sfw: Option<bool>, genres: &[String]) -> ContentRating {
	let lowered: Vec<String> = genres.iter().map(|g| g.trim().to_lowercase()).collect();
	if rating == Some("pornographic") || lowered.iter().any(|g| ADULT_GENRES.contains(&g.as_str()))
	{
		return ContentRating::NSFW;
	}
	if rating == Some("suggestive")
		|| rating == Some("erotica")
		|| sfw == Some(false)
		|| lowered.iter().any(|g| MATURE_GENRES.contains(&g.as_str()))
	{
		return ContentRating::Suggestive;
	}
	ContentRating::Safe
}

fn status_for(comic: &ComicData) -> MangaStatus {
	let text = comic
		.original_status
		.as_deref()
		.or(comic.upload_status.as_deref())
		.unwrap_or("")
		.to_lowercase();
	if text.contains("complet") {
		MangaStatus::Completed
	} else if text.contains("ongoing") {
		MangaStatus::Ongoing
	} else if text.contains("hiatus") {
		MangaStatus::Hiatus
	} else if text.contains("cancel") {
		MangaStatus::Cancelled
	} else {
		MangaStatus::Unknown
	}
}

/// Converts a source timestamp (seconds or milliseconds) to unix seconds.
fn to_unix_seconds(value: i64) -> Option<i64> {
	if value <= 0 {
		None
	} else if value > 1_000_000_000_000 {
		Some(value / 1000)
	} else {
		Some(value)
	}
}

fn card_manga(comic: &ComicData) -> Manga {
	let genres: Vec<String> = comic
		.genres
		.clone()
		.unwrap_or_default()
		.into_iter()
		.filter(|g| !g.trim().is_empty())
		.collect();
	Manga {
		key: comic.id.clone(),
		title: clean(&comic.name),
		cover: Some(comic.url_cover.as_deref().map(abs_url).unwrap_or_default())
			.filter(|c| !c.is_empty()),
		url: Some(abs_url(
			comic
				.url_path
				.as_deref()
				.unwrap_or(&format!("/comic/{}", comic.id)),
		)),
		tags: (!genres.is_empty()).then(|| genres.iter().map(|g| title_case(g)).collect()),
		content_rating: content_rating_for(
			comic.content_rating.as_deref(),
			comic.sfw_result,
			&genres,
		),
		status: status_for(comic),
		..Default::default()
	}
}

fn detail_manga(comic: &ComicData) -> Manga {
	let mut manga = card_manga(comic);
	let authors = node_names(comic.author_nodes.as_ref());
	let artists = node_names(comic.artist_nodes.as_ref());
	manga.authors = (!authors.is_empty()).then(|| authors.clone());
	manga.artists = (!artists.is_empty()).then_some(artists);
	let mut genres: Vec<String> = comic.genres.clone().unwrap_or_default();
	genres.extend(comic.demographics.clone().unwrap_or_default());
	let tags = comic
		.tags
		.clone()
		.filter(|t| !t.is_empty())
		.unwrap_or_else(|| node_names(comic.tag_nodes.as_ref()));
	genres.extend(tags);
	let mut seen: Vec<String> = Vec::new();
	let named: Vec<String> = genres
		.into_iter()
		.filter(|g| !g.trim().is_empty())
		.filter(|g| {
			let lower = g.to_lowercase();
			if seen.contains(&lower) {
				false
			} else {
				seen.push(lower);
				true
			}
		})
		.map(|g| title_case(&g))
		.collect();
	manga.tags = (!named.is_empty()).then_some(named);
	manga.description = comic
		.summary
		.as_ref()
		.and_then(|s| s.html.as_deref())
		.map(strip_html)
		.filter(|d| !d.is_empty());
	manga
}

fn graphql<T: serde::de::DeserializeOwned>(query: &str, variables: serde_json::Value) -> Result<T> {
	let body = serde_json::json!({ "query": query.trim(), "variables": variables }).to_string();
	let response: GraphQLResponse<T> = Request::post(API_URL)?
		.header("Content-Type", "application/json")
		.header("Accept", "application/json")
		.header("Referer", &format!("{DOMAIN}/"))
		.header("Origin", DOMAIN)
		.body(body)
		.send()?
		.get_json_owned()?;
	let Some(data) = response.data else {
		bail!("XCOMIC returned an empty response");
	};
	Ok(data)
}

#[allow(clippy::too_many_arguments)]
fn browse_select(
	page: i32,
	sortby: &str,
	word: &str,
	inc: &[String],
	exc: &[String],
) -> serde_json::Value {
	serde_json::json!({
		"where": "browse",
		"page": page,
		"size": PAGE_SIZE,
		"init": (page - 1) * PAGE_SIZE,
		"sortby": sortby,
		"word": word,
		"incOLangs": [],
		"incTLangs": ["en"],
		"incGenres": inc,
		"excGenres": exc,
		"incGenresMode": "and",
		"excGenresMode": "or",
		"incTypes": content_types(),
		"incDemographics": [],
		"incContentRatings": content_ratings(),
		"releaseYearMin": serde_json::Value::Null,
		"releaseYearMax": serde_json::Value::Null,
		"origStatus": serde_json::Value::Null,
		"siteStatus": serde_json::Value::Null,
		"chapCount": "",
		"ignoreGlobalULangs": true,
		"ignoreGlobalGenres": true,
		"ignoreGlobalBlocks": true
	})
}

fn browse(
	sortby: &str,
	word: &str,
	page: i32,
	inc: &[String],
	exc: &[String],
) -> Result<Vec<Manga>> {
	let response: BrowseResponse = graphql(
		BROWSE_QUERY,
		serde_json::json!({
			"select": browse_select(page, sortby, word, inc, exc)
		}),
	)?;
	Ok(response
		.get_comic_browse_items
		.unwrap_or_default()
		.iter()
		.filter(|node| {
			node.data
				.url_cover
				.as_deref()
				.map(str::trim)
				.is_some_and(|c| !c.is_empty())
		})
		.map(|node| card_manga(&node.data))
		.collect())
}

const RSS_URL: &str = "https://xcomic.me/rss/added.xml";

/// Decodes the handful of XML entities that appear in RSS titles.
fn decode_entities(input: &str) -> String {
	input
		.replace("&amp;", "&")
		.replace("&lt;", "<")
		.replace("&gt;", ">")
		.replace("&quot;", "\"")
		.replace("&#39;", "'")
		.replace("&apos;", "'")
}

/// Unwraps a `<![CDATA[...]]>` wrapper if present.
fn strip_cdata(value: &str) -> &str {
	value
		.trim()
		.strip_prefix("<![CDATA[")
		.and_then(|rest| rest.strip_suffix("]]>"))
		.unwrap_or(value)
		.trim()
}

/// Returns the inner text of the first `<tag>...</tag>` in `item`.
fn tag_inner(item: &str, tag: &str) -> Option<String> {
	let open = format!("<{tag}");
	let start = item.find(&open)?;
	let after = item[start..].find('>')? + start + 1;
	let close = format!("</{tag}>");
	let end = item[after..].find(&close)? + after;
	Some(decode_entities(strip_cdata(&item[after..end])))
}

/// Reads the `url` attribute from the item's `<enclosure>`.
fn enclosure_url(item: &str) -> Option<String> {
	let start = item.find("<enclosure")?;
	let segment = &item[start..];
	let key = segment.find("url=\"")? + 5;
	let end = segment[key..].find('"')? + key;
	Some(segment[key..end].to_string())
}

/// The comic id is the alphanumeric run right after `/comic/`.
fn comic_id_from(link: &str) -> Option<String> {
	let after = link.split("/comic/").nth(1)?;
	let id: String = after
		.chars()
		.take_while(|c| c.is_ascii_alphanumeric())
		.collect();
	(!id.is_empty()).then_some(id)
}

/// Drops a leading flag emoji (two regional-indicator symbols) and whitespace.
fn strip_leading_flag(title: &str) -> String {
	let trimmed = title.trim_start();
	let mut offset = 0;
	for c in trimmed.chars() {
		if ('\u{1F1E6}'..='\u{1F1FF}').contains(&c) || c.is_whitespace() {
			offset += c.len_utf8();
		} else {
			break;
		}
	}
	trimmed[offset..].trim().to_string()
}

/// The RSS "recently added" feed, fetched and parsed into cards. Unlike the
/// GraphQL browse, this is a plain GET, so it works even when the API is fussy.
fn recently_added() -> Result<Vec<Manga>> {
	let xml = Request::get(RSS_URL)?
		.header("Referer", &format!("{DOMAIN}/"))
		.header("Accept", "application/rss+xml,application/xml,text/xml")
		.send()?
		.get_string()?;

	let mut out: Vec<Manga> = Vec::new();
	let mut seen: Vec<String> = Vec::new();
	for chunk in xml.split("<item").skip(1) {
		let item = chunk.split("</item>").next().unwrap_or(chunk);
		let link = tag_inner(item, "link").unwrap_or_default();
		if !link.contains("/comic/") || !link.contains("-en-") {
			continue;
		}
		let Some(id) = tag_inner(item, "guid")
			.filter(|g| !g.is_empty())
			.or_else(|| comic_id_from(&link))
		else {
			continue;
		};
		if seen.contains(&id) {
			continue;
		}
		let title = strip_leading_flag(&tag_inner(item, "title").unwrap_or_default());
		let cover = enclosure_url(item).map(|u| abs_url(&u)).unwrap_or_default();
		if title.is_empty() || cover.is_empty() {
			continue;
		}
		seen.push(id.clone());
		out.push(Manga {
			key: id,
			title,
			cover: Some(cover),
			url: Some(abs_url(&link)),
			content_rating: ContentRating::Suggestive,
			..Default::default()
		});
	}
	Ok(out)
}

struct XComic;

impl Source for XComic {
	fn new() -> Self {
		Self
	}

	fn get_search_manga_list(
		&self,
		query: Option<String>,
		page: i32,
		filters: Vec<FilterValue>,
	) -> Result<MangaPageResult> {
		let word = query.unwrap_or_default();
		let word = word.trim();
		let mut sortby = String::from("field_score");
		let mut inc: Vec<String> = Vec::new();
		let mut exc: Vec<String> = Vec::new();
		for filter in &filters {
			match filter {
				FilterValue::Sort { id, index, .. } if id == "sort" => {
					if let Some(sort) = SORT_IDS.get(*index as usize) {
						sortby = (*sort).to_string();
					}
				}
				FilterValue::MultiSelect {
					included, excluded, ..
				} => {
					inc.extend(included.iter().cloned());
					exc.extend(excluded.iter().cloned());
				}
				_ => {}
			}
		}
		let entries = browse(&sortby, word, page, &inc, &exc)?;
		Ok(MangaPageResult {
			has_next_page: entries.len() as i32 >= PAGE_SIZE,
			entries,
		})
	}

	fn get_manga_update(
		&self,
		mut manga: Manga,
		needs_details: bool,
		needs_chapters: bool,
	) -> Result<Manga> {
		if needs_details {
			let response: ComicNodeResponse =
				graphql(COMIC_QUERY, serde_json::json!({ "id": manga.key }))?;
			if let Some(node) = response.get_comic_node.as_ref() {
				let mut parsed = detail_manga(&node.data);
				parsed.chapters = manga.chapters.take();
				manga = parsed;
			}
		}
		if needs_chapters {
			manga.chapters = Some(fetch_chapters(&manga.key)?);
		}
		Ok(manga)
	}

	fn get_page_list(&self, _manga: Manga, chapter: Chapter) -> Result<Vec<Page>> {
		let response: ChapterPagesResponse = graphql(
			CHAPTER_PAGES_QUERY,
			serde_json::json!({ "id": chapter.key }),
		)?;
		let pages = response
			.get_chapter_node
			.and_then(|node| node.data)
			.and_then(|data| data.image_urls)
			.unwrap_or_default()
			.iter()
			.map(|url| abs_url(url))
			.filter(|url| !url.is_empty())
			.map(|url| Page {
				content: PageContent::url(url),
				..Default::default()
			})
			.collect();
		Ok(pages)
	}
}

fn fetch_chapters(comic_id: &str) -> Result<Vec<Chapter>> {
	let mut chapters: Vec<Chapter> = Vec::new();
	let first: ChapterListResponse = graphql(CHAPTERS_QUERY, chapters_select(comic_id, 1))?;
	let pages = first
		.chapter_list
		.as_ref()
		.and_then(|result| result.paging.as_ref())
		.and_then(|paging| paging.pages)
		.unwrap_or(1);
	collect_chapters(first, &mut chapters);
	for page in 2..=pages {
		let response: ChapterListResponse =
			graphql(CHAPTERS_QUERY, chapters_select(comic_id, page as i32))?;
		collect_chapters(response, &mut chapters);
	}
	Ok(chapters)
}

fn chapters_select(comic_id: &str, page: i32) -> serde_json::Value {
	serde_json::json!({
		"select": {
			"comic_id": comic_id,
			"page": page,
			"size": CHAPTER_PAGE_SIZE,
			"sortby": "chapter_desc"
		}
	})
}

fn collect_chapters(response: ChapterListResponse, out: &mut Vec<Chapter>) {
	let Some(items) = response.chapter_list.and_then(|result| result.items) else {
		return;
	};
	for node in items {
		let data = node.data;
		if data.db_status.as_deref().unwrap_or("normal") != "normal" {
			continue;
		}
		let number = data.cha_num.or(data.serial).map(|n| n as f32);
		let dname = data.dname.as_deref().map(clean).filter(|t| !t.is_empty());
		let extra = data.title.as_deref().map(clean).filter(|t| !t.is_empty());
		let title = match (dname, extra) {
			(Some(a), Some(b)) if a != b => Some(format!("{a}: {b}")),
			(Some(a), _) => Some(a),
			(None, Some(b)) => Some(b),
			_ => None,
		};
		let date = data
			.date_modify
			.or(data.date_create)
			.or(data.date_public)
			.and_then(to_unix_seconds);
		let mut scanlators = data
			.src_name
			.as_deref()
			.map(|s| vec![title_case(s)])
			.unwrap_or_default();
		if scanlators.is_empty() {
			scanlators = node_names(data.profile_nodes.as_ref());
		}
		if scanlators.is_empty() {
			scanlators = node_names(data.group_nodes.as_ref());
		}
		out.push(Chapter {
			key: data.id.clone(),
			chapter_number: number,
			title,
			date_uploaded: date,
			scanlators: (!scanlators.is_empty()).then_some(scanlators),
			url: data.url_path.as_deref().map(abs_url),
			language: Some("en".into()),
			..Default::default()
		});
	}
}

impl Home for XComic {
	fn get_home(&self) -> Result<HomeLayout> {
		let top_rated = browse("field_score", "", 1, &[], &[]);
		let most_views = browse("views_d030", "", 1, &[], &[]);
		let recently = recently_added();
		let most_chapters = browse("field_chapter", "", 1, &[], &[]);
		let latest: Result<LatestUpdatesResponse> = graphql(
			LATEST_UPDATES_QUERY,
			serde_json::json!({ "select": { "size": PAGE_SIZE } }),
		);

		let mut components: Vec<HomeComponent> = Vec::new();

		if let Ok(entries) = top_rated {
			let entries: Vec<Manga> = entries.into_iter().take(10).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Top Rated".into()),
					subtitle: None,
					value: HomeComponentValue::BigScroller {
						entries,
						auto_scroll_interval: Some(6.0),
					},
				});
			}
		}

		if let Ok(entries) = most_views {
			let entries: Vec<Link> = entries.into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Viewed (30 Days)".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					},
				});
			}
		}

		if let Ok(response) = latest {
			let entries: Vec<MangaWithChapter> = response
				.latest_uploads
				.and_then(|result| result.items)
				.unwrap_or_default()
				.into_iter()
				.filter_map(latest_entry)
				.collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Latest Uploads".into()),
					subtitle: None,
					value: HomeComponentValue::MangaChapterList {
						page_size: None,
						entries,
						listing: None,
					},
				});
			}
		}

		if let Ok(entries) = recently {
			let entries: Vec<Link> = entries.into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Recently Added".into()),
					subtitle: None,
					value: HomeComponentValue::Scroller {
						entries,
						listing: None,
					},
				});
			}
		}

		if let Ok(entries) = most_chapters {
			let entries: Vec<Link> = entries.into_iter().map(Into::into).collect();
			if !entries.is_empty() {
				components.push(HomeComponent {
					title: Some("Most Chapters".into()),
					subtitle: None,
					value: HomeComponentValue::MangaList {
						ranking: true,
						page_size: Some(10),
						entries,
						listing: None,
					},
				});
			}
		}

		Ok(HomeLayout { components })
	}
}

fn latest_entry(item: LatestUploadItem) -> Option<MangaWithChapter> {
	let comic = item.comic?;
	if comic
		.data
		.url_cover
		.as_deref()
		.map(str::trim)
		.unwrap_or("")
		.is_empty()
	{
		return None;
	}
	let chapter_data = item.chapters?.into_iter().next()?.data;
	let manga = card_manga(&comic.data);
	let number = chapter_data
		.cha_num
		.or(chapter_data.serial)
		.map(|n| n as f32);
	let date = chapter_data
		.date_modify
		.or(chapter_data.date_create)
		.or(chapter_data.date_public)
		.and_then(to_unix_seconds);
	Some(MangaWithChapter {
		manga,
		chapter: Chapter {
			key: chapter_data.id.clone(),
			chapter_number: number,
			date_uploaded: date,
			url: chapter_data.url_path.as_deref().map(abs_url),
			language: Some("en".into()),
			..Default::default()
		},
	})
}

impl ListingProvider for XComic {
	fn get_manga_list(&self, listing: Listing, page: i32) -> Result<MangaPageResult> {
		if listing.id == "recently_added" {
			// The RSS feed is one flat list, so it is paged client-side.
			let all = recently_added()?;
			let start = ((page - 1).max(0) * PAGE_SIZE) as usize;
			let entries: Vec<Manga> = all
				.into_iter()
				.skip(start)
				.take(PAGE_SIZE as usize)
				.collect();
			return Ok(MangaPageResult {
				has_next_page: entries.len() as i32 >= PAGE_SIZE,
				entries,
			});
		}
		let entries = browse(&listing.id, "", page, &[], &[])?;
		Ok(MangaPageResult {
			has_next_page: entries.len() as i32 >= PAGE_SIZE,
			entries,
		})
	}
}

impl ImageRequestProvider for XComic {
	fn get_image_request(&self, url: String, _context: Option<PageContext>) -> Result<Request> {
		Ok(Request::get(url)?
			.header("Referer", &format!("{DOMAIN}/"))
			.header("Origin", DOMAIN))
	}
}

impl DeepLinkHandler for XComic {
	fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {
		let Some(rest) = url.split("/comic/").nth(1) else {
			return Ok(None);
		};
		let id: String = rest
			.chars()
			.take_while(|c| c.is_ascii_alphanumeric())
			.collect();
		if id.is_empty() {
			return Ok(None);
		}
		Ok(Some(DeepLinkResult::Manga { key: id }))
	}
}

register_source!(
	XComic,
	Home,
	ListingProvider,
	ImageRequestProvider,
	DeepLinkHandler
);
