use crate::helpers::{
	content_rating, cover_url, display_title, parse_status, strip_html, tag_names, viewer,
};
use aidoku::{
	AidokuError, ContentRating, Manga, Result,
	alloc::{String, Vec},
	prelude::*,
};
use serde::Deserialize;

pub const BASE_URL: &str = "https://mkissa.to";
pub const API_URL: &str = "https://api.mkissa.net/api";
pub const CDN_URL: &str = "https://wp.youtube-anime.com";

pub const THUMBNAIL_CDN: &str = "https://wp.youtube-anime.com/aln.youtube-anime.com/";
pub const DEFAULT_IMAGE_SERVER: &str = "https://ytimgf.youtube-anime.com/";

pub const PAGE_SIZE: i32 = 20;

pub const POPULAR_QUERY: &str = "query($type: VaildPopularTypeEnumType!, $size: Int!, $page: Int, $dateRange: Int, $allowAdult: Boolean, $allowUnknown: Boolean) { queryPopular(type: $type, size: $size, dateRange: $dateRange, page: $page, allowAdult: $allowAdult, allowUnknown: $allowUnknown) { total recommendations { anyCard { _id name englishName thumbnail isAdult description genres status } } } }";

pub const TRENDING_QUERY: &str = "query($pageSearch: queryPageInput!) { queryLatestPageStatus(pageSearch: $pageSearch) { total recommendations { anyCard { _id name englishName thumbnail isAdult } } } }";

pub const RANDOM_QUERY: &str = "query($format: String!, $allowAdult: Boolean, $denyEcchi: Boolean, $slot: Int) { queryRandomRecommendation(format: $format, allowAdult: $allowAdult, denyEcchi: $denyEcchi, slot: $slot) { _id name englishName thumbnail isAdult } }";

pub const MANGAS_QUERY: &str = "query($search: SearchInput, $limit: Int, $page: Int, $translationType: VaildTranslationTypeMangaEnumType, $countryOrigin: VaildCountryOriginEnumType) { mangas(search: $search, limit: $limit, page: $page, translationType: $translationType, countryOrigin: $countryOrigin) { pageInfo { total } edges { _id name englishName thumbnail isAdult lastChapterInfo lastChapterDate } } }";

pub const ADAPTATIONS_QUERY: &str = "query($page: Int, $limit: Int, $translationType: VaildTranslationTypeMangaEnumType, $allowAdult: Boolean, $allowUnknown: Boolean, $denyEcchi: Boolean) { seasonAdaptationMangas(page: $page, limit: $limit, translationType: $translationType, allowAdult: $allowAdult, allowUnknown: $allowUnknown, denyEcchi: $denyEcchi) { pageInfo { total } edges { _id name englishName thumbnail isAdult } } }";

pub const DETAILS_QUERY: &str = "query($id: String!) { manga(_id: $id) { _id name englishName thumbnail isAdult description authors genres tags status type countryOfOrigin } }";

pub const CHAPTERS_QUERY: &str = "query($id: String!, $showId: String!) { manga(_id: $id) { _id availableChaptersDetail } episodeInfos(showId: $showId, episodeNumStart: 0, episodeNumEnd: 9999) { episodeIdNum notes uploadDates } }";

#[derive(Deserialize)]
pub struct GraphQLResponse<T> {
	pub data: Option<T>,
	pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
pub struct GraphQLError {
	pub message: String,
}

#[derive(Deserialize, Default)]
pub struct DateParts {
	pub year: Option<i32>,
	pub month: Option<i32>,
	pub date: Option<i32>,
	pub hour: Option<i32>,
	pub minute: Option<i32>,
	pub second: Option<i32>,
}

#[derive(Deserialize)]
pub struct Translations<T> {
	pub sub: Option<T>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LastChapter {
	pub chapter_string: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaCard {
	#[serde(rename = "_id")]
	pub id: String,
	#[serde(default)]
	pub name: String,
	pub english_name: Option<String>,
	pub thumbnail: Option<String>,
	pub is_adult: Option<bool>,
	pub description: Option<String>,
	pub genres: Option<Vec<String>>,
	pub status: Option<String>,
	pub last_chapter_info: Option<Translations<LastChapter>>,
	pub last_chapter_date: Option<Translations<DateParts>>,
}

impl MangaCard {
	// The big scroller shows what the popular feed already carries
	pub fn into_featured(self) -> Result<Manga> {
		let tags = tag_names(self.genres.clone().unwrap_or_default());
		Ok(Manga {
			description: self
				.description
				.as_deref()
				.map(strip_html)
				.filter(|description| !description.is_empty()),
			status: parse_status(self.status.as_deref()),
			content_rating: content_rating(&tags, self.is_adult),
			tags: (!tags.is_empty()).then_some(tags),
			..Manga::try_from(self)?
		})
	}
}

impl TryFrom<MangaCard> for Manga {
	type Error = AidokuError;

	fn try_from(card: MangaCard) -> Result<Self> {
		let title = display_title(card.english_name.as_deref(), &card.name)?;
		if card.id.trim().is_empty() {
			bail!("Series has no key");
		}
		Ok(Manga {
			title,
			cover: cover_url(card.thumbnail.as_deref()),
			content_rating: if card.is_adult == Some(true) {
				ContentRating::NSFW
			} else {
				ContentRating::Unknown
			},
			key: card.id,
			..Default::default()
		})
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recommendation {
	pub any_card: Option<MangaCard>,
}

#[derive(Deserialize)]
pub struct Recommendations {
	pub total: Option<i32>,
	pub recommendations: Vec<Recommendation>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PopularData {
	pub query_popular: Recommendations,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TrendingData {
	pub query_latest_page_status: Recommendations,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RandomData {
	pub query_random_recommendation: Option<Vec<MangaCard>>,
}

#[derive(Deserialize)]
pub struct PageInfo {
	pub total: Option<i32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaEdges {
	pub page_info: PageInfo,
	pub edges: Vec<MangaCard>,
}

#[derive(Deserialize)]
pub struct SearchData {
	pub mangas: MangaEdges,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AdaptationsData {
	pub season_adaptation_mangas: MangaEdges,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MangaDetail {
	#[serde(default)]
	pub name: String,
	pub english_name: Option<String>,
	pub thumbnail: Option<String>,
	pub is_adult: Option<bool>,
	pub description: Option<String>,
	pub authors: Option<Vec<String>>,
	pub genres: Option<Vec<String>>,
	pub tags: Option<Vec<String>>,
	pub status: Option<String>,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	pub country_of_origin: Option<String>,
}

impl MangaDetail {
	pub fn into_details(self, key: &str) -> Result<Manga> {
		let tags = tag_names(
			self.genres
				.unwrap_or_default()
				.into_iter()
				.chain(self.tags.unwrap_or_default())
				.collect(),
		);
		let mut authors: Vec<String> = Vec::new();
		for author in self.authors.unwrap_or_default() {
			let name = author.trim();
			if !name.is_empty() && !authors.iter().any(|existing| existing == name) {
				authors.push(name.into());
			}
		}
		Ok(Manga {
			key: key.into(),
			title: display_title(self.english_name.as_deref(), &self.name)?,
			cover: cover_url(self.thumbnail.as_deref()),
			description: self
				.description
				.as_deref()
				.map(strip_html)
				.filter(|description| !description.is_empty()),
			authors: (!authors.is_empty()).then_some(authors),
			status: parse_status(self.status.as_deref()),
			content_rating: content_rating(&tags, self.is_adult),
			viewer: viewer(
				self.kind.as_deref(),
				self.country_of_origin.as_deref(),
				&tags,
			),
			url: Some(format!("{BASE_URL}/manga/{key}")),
			tags: (!tags.is_empty()).then_some(tags),
			..Default::default()
		})
	}
}

#[derive(Deserialize)]
pub struct DetailsData {
	pub manga: Option<MangaDetail>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChaptersManga {
	pub available_chapters_detail: Option<Translations<Vec<String>>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EpisodeInfo {
	pub episode_id_num: serde_json::Value,
	pub notes: Option<String>,
	pub upload_dates: Option<Translations<String>>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChaptersData {
	pub manga: Option<ChaptersManga>,
	pub episode_infos: Option<Vec<EpisodeInfo>>,
}

#[derive(Deserialize)]
#[serde(untagged)]
pub enum PictureUrl {
	Str(String),
	Obj { url: Option<String> },
}

impl PictureUrl {
	pub fn url(&self) -> Option<&str> {
		match self {
			PictureUrl::Str(s) => Some(s),
			PictureUrl::Obj { url } => url.as_deref(),
		}
	}
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChapterPageEdge {
	// The reader names the image host `serverUrl`; older payloads used `pictureUrlHead`
	pub server_url: Option<String>,
	pub picture_url_head: Option<String>,
	pub picture_urls: Option<Vec<PictureUrl>>,
}

impl ChapterPageEdge {
	pub fn image_host(&self) -> Option<&str> {
		self.server_url
			.as_deref()
			.or(self.picture_url_head.as_deref())
	}
}

#[derive(Deserialize)]
pub struct ChapterPages {
	pub edges: Vec<ChapterPageEdge>,
}
