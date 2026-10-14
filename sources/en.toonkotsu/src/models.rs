use crate::{
	api::BASE_URL,
	helpers::{clean_text, cover_url, is_unlisted_tag, manga_url, rating_id, split_label},
};
use aidoku::{
	Chapter, ContentRating, Manga, MangaStatus, Viewer,
	alloc::{String, Vec, collections::BTreeSet, format, string::ToString, vec},
};
use serde::Deserialize;

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Series {
	/// The id every url and request uses.
	pub hid: Option<String>,
	pub slug: Option<String>,
	pub title: String,
	pub description: Option<String>,
	#[serde(rename = "type")]
	pub kind: Option<String>,
	pub tags: Option<Vec<String>>,
	pub spoiler_tags: Option<Vec<String>>,
	pub status: Option<String>,
	pub cover: Option<String>,
	pub authors: Option<Vec<String>>,
	pub adult: Option<bool>,
}

impl Series {
	fn key(&self) -> Option<String> {
		self.hid
			.as_deref()
			.map(str::trim)
			.filter(|hid| !hid.is_empty())
			.map(Into::into)
	}

	fn content_rating(&self) -> ContentRating {
		match rating_id(self) {
			"safe" => ContentRating::Safe,
			"suggestive" => ContentRating::Suggestive,
			_ => ContentRating::NSFW,
		}
	}

	/// What a card shows; the rest arrives with the details.
	pub fn into_manga(self) -> Option<Manga> {
		let key = self.key()?;
		Some(Manga {
			url: Some(manga_url(&key, self.slug.as_deref())),
			cover: cover_url(self.cover.as_deref(), false),
			title: clean_text(&self.title),
			content_rating: self.content_rating(),
			key,
			..Default::default()
		})
	}

	pub fn into_full_manga(self) -> Option<Manga> {
		let key = self.key()?;
		let content_rating = self.content_rating();
		let spoilers = self.spoiler_tags.unwrap_or_default();
		let tags: Vec<String> = self
			.tags
			.unwrap_or_default()
			.into_iter()
			.filter(|tag| !is_unlisted_tag(tag) && !spoilers.contains(tag))
			.collect();
		let authors: Vec<String> = self
			.authors
			.unwrap_or_default()
			.iter()
			.map(|author| clean_text(author))
			.filter(|author| !author.is_empty())
			.collect();
		Some(Manga {
			url: Some(manga_url(&key, self.slug.as_deref())),
			cover: cover_url(self.cover.as_deref(), true),
			title: clean_text(&self.title),
			// The site credits one set of people for both roles.
			artists: (!authors.is_empty()).then(|| authors.clone()),
			authors: (!authors.is_empty()).then_some(authors),
			description: self
				.description
				.as_deref()
				.map(clean_text)
				.filter(|text| !text.is_empty()),
			tags: (!tags.is_empty()).then_some(tags),
			status: match self.status.as_deref() {
				Some("Ongoing") => MangaStatus::Ongoing,
				Some("Complete") => MangaStatus::Completed,
				Some("Hiatus") => MangaStatus::Hiatus,
				Some("Canceled") => MangaStatus::Cancelled,
				_ => MangaStatus::Unknown,
			},
			content_rating,
			// Korean and Chinese comics read as webtoons, which is how the site shows them.
			viewer: match self.kind.as_deref() {
				Some("Manhwa") | Some("Manhua") => Viewer::Webtoon,
				Some("Manga") => Viewer::RightToLeft,
				_ => Viewer::Unknown,
			},
			key,
			..Default::default()
		})
	}
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Group {
	pub name: Option<String>,
}

/// One upload of a chapter. A numbered chapter carries its alternates in `alts`.
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Release {
	pub id: i64,
	pub label: Option<String>,
	pub at: Option<i64>,
	pub group: Option<Group>,
}

/// The site numbers a volume either way round.
#[derive(Deserialize)]
#[serde(untagged)]
pub enum Volume {
	Number(f64),
	Text(String),
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ChapterEntry {
	#[serde(flatten)]
	pub release: Release,
	pub n: Option<f64>,
	pub vol: Option<Volume>,
	pub alts: Option<Vec<Release>>,
}

impl Release {
	fn into_chapter(self, number: Option<f64>, volume: Option<&Volume>) -> Chapter {
		let (title, bracketed) = self
			.label
			.as_deref()
			.map(|label| split_label(label, number))
			.unwrap_or_default();
		let scanlator = self
			.group
			.and_then(|group| group.name)
			.map(|name| clean_text(&name))
			.filter(|name| !name.is_empty())
			.or(bracketed);
		Chapter {
			url: Some(format!("{BASE_URL}/chapter/{}", self.id)),
			key: self.id.to_string(),
			// A label that only names the group is no title.
			title: title.filter(|title| {
				scanlator
					.as_deref()
					.is_none_or(|scanlator| !title.eq_ignore_ascii_case(scanlator))
			}),
			chapter_number: number.map(|number| number as f32),
			// The catalogue fills in 0 where a chapter belongs to no volume.
			volume_number: match volume {
				Some(Volume::Number(number)) => Some(*number as f32),
				Some(Volume::Text(text)) => text.trim().parse().ok(),
				None => None,
			}
			.filter(|volume| *volume > 0.0),
			// The catalogue times its uploads in milliseconds.
			date_uploaded: self.at.map(|at| at / 1000),
			scanlators: scanlator.map(|name| vec![name]),
			language: Some("en".into()),
			..Default::default()
		}
	}
}

impl ChapterEntry {
	/// The chapter's own upload, leaving out its alternates.
	pub fn into_chapter(self) -> Chapter {
		self.release.into_chapter(self.n, self.vol.as_ref())
	}

	/// The chapter's own upload, then the alternates, with only the first from each group:
	/// an alternate can be the same group's upload mirrored from elsewhere.
	pub fn into_chapters(self) -> Vec<Chapter> {
		let ChapterEntry {
			release,
			n,
			vol,
			alts,
		} = self;
		let mut chapters: Vec<Chapter> = Vec::new();
		for release in core::iter::once(release).chain(alts.unwrap_or_default()) {
			let chapter = release.into_chapter(n, vol.as_ref());
			let group = chapter.scanlators.as_ref().and_then(|names| names.first());
			let seen = chapters.iter().any(|kept| {
				let kept = kept.scanlators.as_ref().and_then(|names| names.first());
				match (kept, group) {
					(Some(kept), Some(group)) => kept.eq_ignore_ascii_case(group),
					(None, None) => true,
					_ => false,
				}
			});
			if !seen {
				chapters.push(chapter);
			}
		}
		chapters
	}
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SeriesList {
	pub items: Option<Vec<Series>>,
	pub limit: Option<i64>,
	pub total: Option<i64>,
	pub has_next_page: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LatestItem {
	pub chapter: ChapterEntry,
	pub series: Series,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct LatestList {
	pub items: Option<Vec<LatestItem>>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SeriesDetail {
	pub series: Series,
	pub chapters: Option<Vec<ChapterEntry>>,
}

/// Every upload once: a release can be listed under more than one chapter, and the site
/// draws it once, so its id decides rather than where it was listed.
pub fn chapter_list(entries: Vec<ChapterEntry>) -> Vec<Chapter> {
	let mut seen: BTreeSet<String> = BTreeSet::new();
	entries
		.into_iter()
		.flat_map(ChapterEntry::into_chapters)
		.filter(|chapter| seen.insert(chapter.key.clone()))
		.collect()
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Facet {
	pub name: String,
	pub category: Option<String>,
	pub is_adult: Option<bool>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct Facets {
	pub genres: Option<Vec<Facet>>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderKey {
	pub key: String,
	/// The site's own clock, which signing uses so a wrong device clock cannot reject it.
	pub now: i64,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderPage {
	pub src: String,
	pub k: Option<String>,
	pub n: Option<String>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderChapter {
	pub pages: Option<Vec<ReaderPage>>,
}

#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderResponse {
	/// Absent or empty means the page keys are used as served.
	pub wrap: Option<String>,
	pub chapter: ReaderChapter,
}

/// The body of a refused read: a human check, or a wait the site names in seconds.
#[derive(Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct ReaderLimit {
	pub code: Option<String>,
	pub reason: Option<String>,
	pub transient: Option<bool>,
	pub retry_after_sec: Option<i64>,
}
