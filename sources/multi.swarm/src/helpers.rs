use crate::api::{API_URL, BASE_URL};
use aes::Aes256;
use aidoku::{
	alloc::{String, Vec, format},
	helpers::uri::encode_uri_component,
	imports::std::current_date,
};
use cbc::cipher::{BlockEncryptMut, KeyIvInit, block_padding::Pkcs7};
use sha2::{Digest, Sha256};

const LANGUAGE_ALIASES: [(&str, &str); 4] = [
	("es-la", "es-419"),
	("kr", "ko"),
	("pt-br", "pt-BR"),
	("zh-hk", "zh"),
];

const PAYLOAD_SECRET: &str = "e3bca6f9a7e64e369ddc450bb7ff295b3d78e7d4fcb148c10d10ae9385476a5f";

pub enum Link {
	Comic(String),
	Chapter(String, String, String),
	Novel(String),
	NovelChapter(String, String),
}

pub fn app_language(code: &str) -> &str {
	LANGUAGE_ALIASES
		.iter()
		.find(|(site, _)| *site == code)
		.map(|(_, app)| *app)
		.unwrap_or(code)
}

pub fn names(list: impl IntoIterator<Item = String>) -> Option<Vec<String>> {
	let mut names: Vec<String> = Vec::new();
	for name in list {
		let name = name.trim();
		if !name.is_empty() && !names.iter().any(|kept| kept == name) {
			names.push(name.into());
		}
	}
	(!names.is_empty()).then_some(names)
}

pub fn cover_url(cover: &str) -> Option<String> {
	if cover.is_empty() {
		None
	} else if let Some(path) = cover.strip_prefix("https://cum.swarm.ws/") {
		Some(format!("{BASE_URL}/covers/{path}"))
	} else if cover.starts_with(BASE_URL) {
		Some(cover.into())
	} else {
		Some(format!(
			"{API_URL}/image-proxy?url={}",
			encode_uri_component(cover)
		))
	}
}

pub fn parse_link(url: &str) -> Option<Link> {
	let path = url.trim().strip_prefix(BASE_URL)?;
	let (path, rank) = path.split_once('?').unwrap_or((path, "a"));
	let mut parts = path.trim_matches('/').split('/');
	let kind = parts.next()?;
	let slug = String::from(parts.next().filter(|slug| !slug.is_empty())?);
	match (kind, parts.next(), parts.next()) {
		("comic", None, _) => Some(Link::Comic(slug)),
		("comic", Some("chapter"), Some(number)) => {
			Some(Link::Chapter(slug, number.into(), rank.into()))
		}
		("novel", None, _) => Some(Link::Novel(slug)),
		("novel", Some("chapter"), Some(number)) => Some(Link::NovelChapter(slug, number.into())),
		_ => None,
	}
}

pub fn encrypt_payload(json: &str) -> String {
	let key = Sha256::digest(PAYLOAD_SECRET);
	let seed = Sha256::digest(format!("{json}{}", current_date()));
	let iv = &seed[..16];
	let encrypted = cbc::Encryptor::<Aes256>::new(&key, iv.into())
		.encrypt_padded_vec_mut::<Pkcs7>(json.as_bytes());
	let hex = |bytes: &[u8]| {
		bytes
			.iter()
			.map(|byte| format!("{byte:02x}"))
			.collect::<Vec<_>>()
			.concat()
	};
	format!("{}:{}", hex(iv), hex(&encrypted))
}
