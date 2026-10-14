use crate::{
	api::{self, BASE_URL},
	models::{ReaderKey, ReaderLimit, ReaderResponse},
};
use aidoku::{
	Result,
	alloc::{String, Vec, format, string::ToString},
	imports::net::{Request, Response},
	prelude::*,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ctr::cipher::{KeyIvInit, StreamCipher};
use hmac::{Mac, SimpleHmac};
use sha2::{Digest, Sha256};

/// The reader's signing secret, which the site ships in its player bundle.
const READER_SECRET: &str = "NO57DoQRq7oxY3MCO5_SNw4_3vFRsUq7";

type HmacSha256 = SimpleHmac<Sha256>;

fn decode(value: &str) -> Result<Vec<u8>> {
	URL_SAFE_NO_PAD
		.decode(value)
		.map_err(|_| error!("Toonkotsu sent reader data that is not base64url"))
}

fn xor_cycle(data: &[u8], mask: &[u8]) -> Vec<u8> {
	data.iter()
		.enumerate()
		.map(|(index, byte)| byte ^ mask[index % mask.len()])
		.collect()
}

/// The served key is masked with a digest of the reader secret; unmasked it is the key
/// that signs chapter requests and unwraps the page keys.
pub struct Signer {
	key: Vec<u8>,
	now: i64,
}

impl Signer {
	fn new(response: &ReaderKey) -> Result<Self> {
		let mask = Sha256::digest(format!("sign:{READER_SECRET}").as_bytes());
		Ok(Self {
			key: xor_cycle(&decode(&response.key)?, &mask),
			now: response.now,
		})
	}

	fn sign(&self, message: &str) -> Vec<u8> {
		let mut mac = HmacSha256::new_from_slice(&self.key).expect("hmac accepts any key length");
		mac.update(message.as_bytes());
		mac.finalize().into_bytes().to_vec()
	}

	/// The site times its signatures in milliseconds and takes a nonce it never checks for
	/// randomness, so one derived from the request stands in for a random one.
	fn header(&self, path: &str) -> String {
		let time = self.now.to_string();
		let nonce =
			URL_SAFE_NO_PAD.encode(&Sha256::digest(format!("{time}.{path}").as_bytes())[..12]);
		let signature = URL_SAFE_NO_PAD.encode(self.sign(&format!("{time}.{nonce}.{path}")));
		format!("v1.{time}.{nonce}.{}", &signature[..32])
	}

	/// A "v1" wrapped chapter masks each page key with a signature of that page's nonce.
	fn unwrap_page_key(&self, key: &str, nonce: &str) -> Result<String> {
		Ok(URL_SAFE_NO_PAD.encode(xor_cycle(
			&decode(key)?,
			&self.sign(&format!("page.{nonce}")),
		)))
	}
}

/// One page of a chapter, with the key and nonce that decrypt it when it is encrypted.
pub struct PageSource {
	pub url: String,
	pub key: Option<String>,
	pub nonce: Option<String>,
}

/// Guests are asked to pass a human check before reading on, which only the site's own
/// reader can run; a temporary limit names its own wait instead.
fn check_limit(response: Response) -> Result<Response> {
	if response.status_code() != 429 {
		return Ok(response);
	}
	let limit = response.get_json_owned::<ReaderLimit>().unwrap_or_default();
	if limit.code.as_deref() != Some("manga-limit") {
		bail!("Toonkotsu refused to load this chapter (status 429)");
	}
	let waits = limit.reason.as_deref() == Some("wait");
	if limit.transient == Some(true) || waits {
		let seconds = limit
			.retry_after_sec
			.unwrap_or(if waits { 600 } else { 10 });
		bail!("Toonkotsu is limiting reads right now. Try again in {seconds} seconds.");
	}
	bail!(
		"Toonkotsu wants you to confirm you're human. Open this chapter in the web view, pass the check, then reopen it."
	);
}

pub fn page_sources(chapter_key: &str) -> Result<Vec<PageSource>> {
	let request = |path: &str| -> Result<Request> {
		Ok(Request::get(format!("{BASE_URL}{path}"))?
			.header("Accept", "application/json")
			.header("Referer", &format!("{BASE_URL}/chapter/{chapter_key}")))
	};
	// The reader refuses a request without the guest session cookie, which only this
	// call hands out; it is the one the site's pages make on load.
	request("/api/auth/me")?.send()?;
	let key: ReaderKey = api::parse(check_limit(request("/api/manga/key")?.send()?)?)?;
	let signer = Signer::new(&key)?;

	let path = format!("/api/manga/chapter/{chapter_key}");
	let response = request(&path)?
		.header("x-sig", &signer.header(&path))
		.send()?;
	let chapter: ReaderResponse = api::parse(check_limit(response)?)?;

	let wrapped = match chapter.wrap.as_deref() {
		None | Some("") => false,
		Some("v1") => true,
		Some(version) => bail!("This source does not support reader encryption {version} yet"),
	};
	let pages = chapter.chapter.pages.unwrap_or_default();
	if pages.is_empty() {
		bail!("No pages found for this chapter");
	}
	pages
		.into_iter()
		.map(|page| {
			let url = crate::helpers::absolute_url(&page.src);
			let (Some(key), Some(nonce)) = (page.k, page.n) else {
				return Ok(PageSource {
					url,
					key: None,
					nonce: None,
				});
			};
			let key = if wrapped {
				signer.unwrap_page_key(&key, &nonce)?
			} else {
				key
			};
			Ok(PageSource {
				url,
				key: Some(key),
				nonce: Some(nonce),
			})
		})
		.collect()
}

/// Pages are AES-CTR encrypted, with the first eight bytes of the nonce followed by a
/// 64-bit block counter.
pub fn decrypt_page(data: &mut [u8], key: &str, nonce: &str) -> Result<()> {
	let key = decode(key)?;
	let nonce = decode(nonce)?;
	if nonce.len() < 8 {
		bail!("Toonkotsu sent a page nonce shorter than eight bytes");
	}
	let mut counter = [0u8; 16];
	counter[..8].copy_from_slice(&nonce[..8]);
	match key.len() {
		16 => ctr::Ctr64BE::<aes::Aes128>::new(key.as_slice().into(), &counter.into())
			.apply_keystream(data),
		24 => ctr::Ctr64BE::<aes::Aes192>::new(key.as_slice().into(), &counter.into())
			.apply_keystream(data),
		32 => ctr::Ctr64BE::<aes::Aes256>::new(key.as_slice().into(), &counter.into())
			.apply_keystream(data),
		length => bail!("Toonkotsu sent a {length}-byte page key, which is not an AES key size"),
	}
	Ok(())
}
