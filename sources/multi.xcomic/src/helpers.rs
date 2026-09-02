use crate::models::{ChapterData, ComicData, NamedData, Node};
use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Viewer,
	alloc::{String, Vec, format, string::ToString, vec},
};

pub const PORNOGRAPHIC_GENRES: &[&str] = &["adult", "hentai", "pornographic", "smut"];
const SUGGESTIVE_GENRES: &[&str] = &["ecchi", "erotica", "mature", "yaoi", "yuri"];

// Single source of truth for the search filters and the exclusion settings,
// both of which are built from these so they cannot drift apart.
/// `(id, title)`. The site offers these for both original and translated
/// language filtering; `_t` is its own catch-all id.
pub const LANGUAGES: &[(&str, &str)] = &[
	("en", "English"),
	("zh", "Chinese"),
	("ja", "Japanese"),
	("ko", "Korean"),
	("ar", "Arabic"),
	("de", "German"),
	("es", "Spanish"),
	("es_419", "Spanish (Latin America)"),
	("fr", "French"),
	("hi", "Hindi"),
	("id", "Indonesian"),
	("it", "Italian"),
	("pl", "Polish"),
	("pt", "Portuguese"),
	("pt_br", "Portuguese (Brazil)"),
	("ru", "Russian"),
	("th", "Thai"),
	("tr", "Turkish"),
	("uk", "Ukrainian"),
	("vi", "Vietnamese"),
	("zh_hk", "Chinese (Cantonese)"),
	("zh_tw", "Chinese (Traditional)"),
	("_t", "Other"),
];

/// `(id, title)`. Fallback when the live list cannot be fetched.
pub const GENRES: &[(&str, &str)] = &[
	("action", "Action"),
	("adventure", "Adventure"),
	("comedy", "Comedy"),
	("cooking", "Cooking"),
	("doujinshi", "Doujinshi"),
	("drama", "Drama"),
	("ecchi", "Ecchi"),
	("fantasy", "Fantasy"),
	("gender_bender", "Gender Bender"),
	("harem", "Harem"),
	("historical", "Historical"),
	("horror", "Horror"),
	("isekai", "Isekai"),
	("josei", "Josei"),
	("magic", "Magic"),
	("martial_arts", "Martial Arts"),
	("mature", "Mature"),
	("mecha", "Mecha"),
	("medical", "Medical"),
	("military", "Military"),
	("music", "Music"),
	("mystery", "Mystery"),
	("psychological", "Psychological"),
	("romance", "Romance"),
	("school_life", "School Life"),
	("sci_fi", "Sci-Fi"),
	("seinen", "Seinen"),
	("shoujo", "Shoujo"),
	("shounen", "Shounen"),
	("slice_of_life", "Slice of Life"),
	("smut", "Smut"),
	("sports", "Sports"),
	("supernatural", "Supernatural"),
	("thriller", "Thriller"),
	("tragedy", "Tragedy"),
	("webtoon", "Webtoon"),
	("yaoi", "Yaoi"),
	("yuri", "Yuri"),
];

/// `(id, title)`. Also used to split formats out of the live genre list, which
/// the site serves as one combined group.
pub const FORMATS: &[(&str, &str)] = &[
	("4_koma", "4 Koma"),
	("adaptation", "Adaptation"),
	("anthology", "Anthology"),
	("award_winning", "Award Winning"),
	("doujinshi", "Doujinshi"),
	("fan_colored", "Fan Colored"),
	("full_color", "Full Color"),
	("long_strip", "Long Strip"),
	("official_colored", "Official Colored"),
	("oneshot", "Oneshot"),
	("web_comic", "Web Comic"),
	("webtoon", "Webtoon"),
];

pub fn absolute_url(base_url: &str, url: &str) -> String {
	let url = url.trim();
	if url.is_empty() {
		String::new()
	} else if url.starts_with("http://") || url.starts_with("https://") {
		url.into()
	} else if let Some(rest) = url.strip_prefix("//") {
		format!("https://{rest}")
	} else if url.starts_with('/') {
		format!("{base_url}{url}")
	} else {
		format!("{base_url}/{url}")
	}
}

fn clean(text: &str) -> String {
	text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn title_case(value: &str) -> String {
	let mut output = String::with_capacity(value.len());
	let mut capitalize = true;
	for character in value.replace('_', " ").chars() {
		if capitalize && character.is_alphabetic() {
			output.extend(character.to_uppercase());
			capitalize = false;
		} else {
			output.push(character);
			capitalize = character == ' ';
		}
	}
	output
}

fn node_names(nodes: Vec<Node<Option<NamedData>>>) -> Vec<String> {
	nodes
		.into_iter()
		.filter_map(|node| node.data)
		.filter_map(|data| data.name)
		.map(|name| clean(&name))
		.filter(|name| !name.is_empty())
		.collect()
}

// The API is inconsistent about genre casing.
fn has_any(genres: &[String], list: &[&str]) -> bool {
	genres
		.iter()
		.any(|genre| list.contains(&genre.trim().to_ascii_lowercase().as_str()))
}

pub fn is_pornographic(rating: Option<&str>, genres: Option<&[String]>) -> bool {
	rating == Some("pornographic")
		|| genres.is_some_and(|genres| has_any(genres, PORNOGRAPHIC_GENRES))
}

fn content_rating(rating: Option<&str>, genres: &[String]) -> ContentRating {
	if is_pornographic(rating, Some(genres)) {
		ContentRating::NSFW
	} else if matches!(rating, Some("suggestive" | "erotica")) || has_any(genres, SUGGESTIVE_GENRES)
	{
		ContentRating::Suggestive
	} else {
		ContentRating::Safe
	}
}

fn status(original_status: Option<&str>, upload_status: Option<&str>) -> MangaStatus {
	let status = original_status.or(upload_status).unwrap_or_default();
	if status.contains("completed") {
		MangaStatus::Completed
	} else if status.contains("ongoing") {
		MangaStatus::Ongoing
	} else if status.contains("hiatus") {
		MangaStatus::Hiatus
	} else if status.contains("cancelled") {
		MangaStatus::Cancelled
	} else {
		MangaStatus::Unknown
	}
}

fn viewer(read_direction: Option<&str>, kind: Option<&str>, genres: &[String]) -> Viewer {
	match read_direction {
		Some("ttb") => Viewer::Webtoon,
		Some("rtl") => Viewer::RightToLeft,
		Some("ltr") => Viewer::LeftToRight,
		_ if matches!(kind, Some("manhwa" | "manhua" | "webtoon"))
			|| genres.iter().any(|genre| genre == "webtoon") =>
		{
			Viewer::Webtoon
		}
		_ if kind == Some("manga") => Viewer::RightToLeft,
		_ => Viewer::Unknown,
	}
}

/// Maps whatever the query returned; absent fields simply stay empty.
pub fn manga_from_data(mut comic: ComicData, base_url: &str) -> Manga {
	let mut raw_tags = comic.genres.take().unwrap_or_default();
	raw_tags.extend(comic.demographics.take().unwrap_or_default());
	raw_tags.extend(comic.tags.take().unwrap_or_default());
	if let Some(nodes) = comic.tag_nodes.take() {
		raw_tags.extend(node_names(nodes));
	}
	// Rating and viewer read the site's own lowercase ids, so derive them before
	// the tags are title cased for display.
	let rating = content_rating(comic.content_rating.as_deref(), &raw_tags);
	let preferred_viewer = viewer(
		comic.read_direction.as_deref(),
		comic.kind.as_deref(),
		&raw_tags,
	);

	let mut seen = Vec::new();
	let tags: Vec<String> = raw_tags
		.into_iter()
		.filter(|tag| !tag.trim().is_empty())
		.filter(|tag| {
			let normalized = tag.to_ascii_lowercase();
			let unseen = !seen.contains(&normalized);
			if unseen {
				seen.push(normalized);
			}
			unseen
		})
		.map(|tag| title_case(&tag))
		.collect();

	let cover = comic
		.url_cover
		.take()
		.map(|url| absolute_url(base_url, &url))
		.filter(|url| !url.is_empty());
	let url = comic
		.url_path
		.take()
		.map(|url| absolute_url(base_url, &url))
		.unwrap_or_else(|| format!("{base_url}/comic/{}", comic.id));
	let authors = comic
		.author_nodes
		.take()
		.map(node_names)
		.filter(|names| !names.is_empty());
	let artists = comic
		.artist_nodes
		.take()
		.map(node_names)
		.filter(|names| !names.is_empty());
	let description = comic
		.summary
		.take()
		.and_then(|summary| summary.text)
		.map(|summary| summary.trim().to_string())
		.filter(|description| !description.is_empty());
	let publish_status = status(
		comic.original_status.as_deref(),
		comic.upload_status.as_deref(),
	);
	let title = clean(&comic.name);

	Manga {
		key: comic.id,
		title,
		cover,
		url: Some(url),
		authors,
		artists,
		tags: (!tags.is_empty()).then_some(tags),
		description,
		status: publish_status,
		content_rating: rating,
		viewer: preferred_viewer,
		..Default::default()
	}
}

fn format_number(value: f64) -> String {
	let whole = value as i64;
	if whole as f64 == value {
		format!("{whole}")
	} else {
		format!("{value}")
	}
}

/// `Manhwa · Ch. 12` subtitle, from the type and the latest chapter.
pub fn type_and_chapter(comic: &ComicData) -> Option<String> {
	let kind = comic.kind.as_deref().map(title_case);
	let chapter = comic
		.last_chapter
		.as_ref()
		.and_then(|nodes| nodes.first())
		.and_then(|node| node.data.cha_num.or(node.data.serial))
		.map(|number| format!("Ch. {}", format_number(number)));
	match (kind, chapter) {
		(Some(kind), Some(chapter)) => Some(format!("{kind} · {chapter}")),
		(Some(only), None) | (None, Some(only)) => Some(only),
		(None, None) => None,
	}
}

/// `(comic, chapter)` ids from a `/comic/{id}-slug[/{id}-slug]` url.
pub fn parse_comic_url(url: &str) -> Option<(String, Option<String>)> {
	fn id(segment: &str) -> Option<String> {
		segment
			.split('-')
			.next()
			.filter(|id| !id.is_empty())
			.map(Into::into)
	}
	let mut segments = url.split("/comic/").nth(1)?.split('/');
	Some((id(segments.next()?)?, segments.next().and_then(id)))
}

/// Comic id from a pasted url or an `id:<value>` query. A bare id is not
/// accepted, since it would swallow ordinary search terms.
pub fn comic_key_from_query(query: &str) -> Option<String> {
	let query = query.trim();
	if query.contains("/comic/") {
		return parse_comic_url(query).map(|(comic, _)| comic);
	}
	let rest = query
		.get(..3)
		.filter(|prefix| prefix.eq_ignore_ascii_case("id:"))
		.and_then(|_| query.get(3..))?
		.trim();
	let id = rest.split('-').next()?;
	(!id.is_empty() && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')).then(|| id.into())
}

pub fn parse_year(value: &str) -> (Option<i64>, Option<i64>) {
	if let Some((minimum, maximum)) = value.split_once('-') {
		(minimum.trim().parse().ok(), maximum.trim().parse().ok())
	} else {
		let year = value.trim().parse().ok();
		(year, year)
	}
}

fn unix_seconds(timestamp: i64) -> Option<i64> {
	(timestamp > 0).then_some({
		if timestamp > 1_000_000_000_000 {
			timestamp / 1000
		} else {
			timestamp
		}
	})
}

pub fn chapter_from_data(
	mut data: ChapterData,
	base_url: &str,
	language: Option<&str>,
) -> Option<Chapter> {
	if data.db_status.as_deref().unwrap_or("normal") != "normal" {
		return None;
	}
	let display_name = data
		.dname
		.take()
		.map(|name| clean(&name))
		.filter(|name| !name.is_empty());
	let extra_title = data
		.title
		.take()
		.map(|title| clean(&title))
		.filter(|title| !title.is_empty());
	let title = match (display_name, extra_title) {
		(Some(display_name), Some(extra_title)) if display_name != extra_title => {
			Some(format!("{display_name}: {extra_title}"))
		}
		(Some(display_name), _) => Some(display_name),
		(None, Some(extra_title)) => Some(extra_title),
		_ => None,
	};
	let mut scanlators = data
		.src_name
		.take()
		.map(|name| title_case(&name))
		.filter(|name| !name.is_empty())
		.map(|name| vec![name])
		.unwrap_or_default();
	if scanlators.is_empty() {
		scanlators = data
			.profile_nodes
			.take()
			.map(node_names)
			.unwrap_or_default();
	}
	if scanlators.is_empty() {
		scanlators = data.group_nodes.take().map(node_names).unwrap_or_default();
	}
	Some(Chapter {
		key: data.id,
		chapter_number: data.cha_num.or(data.serial).map(|number| number as f32),
		title,
		date_uploaded: data
			.date_modify
			.or(data.date_create)
			.or(data.date_public)
			.and_then(unix_seconds),
		scanlators: (!scanlators.is_empty()).then_some(scanlators),
		url: data.url_path.map(|url| absolute_url(base_url, &url)),
		language: language.map(Into::into),
		..Default::default()
	})
}
