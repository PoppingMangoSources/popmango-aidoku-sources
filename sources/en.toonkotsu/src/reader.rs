// reference: https://github.com/PoppingMangoSources/general-extensions-mangago/blob/592613dda3388f0e95c64bc5c42d4cf752ddb42f/src/Toonkotsu/utils.ts
use crate::{
	api::BASE_URL,
	helpers::absolute_url,
	models::{ReaderKey, ReaderLimit, ReaderResponse},
};
use aidoku::{
	Page, PageContent, PageContext, Result,
	alloc::{String, Vec, format, string::ToString},
	imports::{js::WebView, net::Request, std::sleep},
	prelude::*,
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use ctr::cipher::{KeyIvInit, StreamCipher};
use hmac::{Mac, SimpleHmac};
use serde::de::DeserializeOwned;
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
struct Signer {
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

/// Reads go through the source's web view, whose cookies the verify sheet in settings
/// shares, so a human check passed there unlocks the guest session reading uses.
struct Site(WebView);

impl Site {
	fn open(chapter_key: &str) -> Result<Self> {
		let web_view = WebView::new();
		// Loaded under the chapter's own address, as the site's reader is.
		web_view.load_html_blocking(
			"<html></html>",
			Some(&format!("{BASE_URL}/chapter/{chapter_key}")),
		)?;
		Ok(Self(web_view))
	}

	/// The status and body of a same-origin request, which carries the session cookie.
	fn send(&self, method: &str, path: &str, signature: Option<&str>) -> Result<(i32, String)> {
		let signature = signature
			.map(|signature| format!("request.setRequestHeader('x-sig', '{signature}');"))
			.unwrap_or_default();
		let result = self.0.eval(&format!(
			"(() => {{
				const request = new XMLHttpRequest();
				request.open('{method}', '{path}', false);
				request.setRequestHeader('Accept', 'application/json');
				{signature}
				request.send();
				return JSON.stringify([request.status, request.responseText]);
			}})()"
		))?;
		serde_json::from_str(&result).map_err(|_| error!("Toonkotsu's reader could not be reached"))
	}
}

fn read<T: DeserializeOwned>((status, body): (i32, String)) -> Result<T> {
	if !(200..300).contains(&status) {
		bail!("Toonkotsu returned status {status}");
	}
	serde_json::from_str(&body)
		.map_err(|_| error!("Toonkotsu sent reader data this source cannot read"))
}

/// What a refused read needs before it is tried again.
enum Retry {
	Resign,
	Wait(i32),
	/// The human check, which belongs to the guest session rather than to the reader.
	Verify,
}

fn retry_for(limit: ReaderLimit) -> Result<Retry> {
	if limit.code.as_deref() != Some("manga-limit") {
		bail!("Toonkotsu refused to load this chapter (status 429)");
	}
	if limit.resign == Some(true) {
		return Ok(Retry::Resign);
	}
	if limit.transient == Some(true) {
		// The site's reader waits as long as it is told, up to ten seconds.
		return Ok(Retry::Wait(
			limit.retry_after_sec.unwrap_or(2).clamp(1, 10) as i32
		));
	}
	if limit.reason.as_deref() == Some("wait") {
		let seconds = limit.retry_after_sec.unwrap_or(600);
		bail!("Toonkotsu is limiting reads right now. Try again in {seconds} seconds.");
	}
	Ok(Retry::Verify)
}

/// Each encrypted page carries its key and nonce in its context, for the image processor.
pub fn pages(chapter_key: &str) -> Result<Vec<Page>> {
	let site = Site::open(chapter_key)?;
	// The reader refuses a request without the guest session cookie, which only this
	// call hands out; it is the one the site's pages make on load.
	site.send("GET", "/api/auth/me", None)?;

	let path = format!("/api/manga/chapter/{chapter_key}");
	// As on the site, a refused read is tried twice more with a fresh key.
	let mut retries = 0;
	let mut checks = 0;
	let (signer, chapter) = loop {
		let signer = Signer::new(&read::<ReaderKey>(site.send(
			"GET",
			"/api/manga/key",
			None,
		)?)?)?;
		let response = site.send("GET", &path, Some(&signer.header(&path)))?;
		if response.0 != 429 {
			break (signer, read::<ReaderResponse>(response)?);
		}
		match retry_for(serde_json::from_str(&response.1).unwrap_or_default())? {
			Retry::Verify => {
				checks += 1;
				match checks {
					// A new guest session starts without the check.
					1 => {
						site.0.delete_all_cookies()?;
						site.send("GET", "/api/auth/me", None)?;
					}
					// The site's own reader runs the check, which often passes without a tap,
					// and unlocks the session; it is given a few seconds to.
					2 => {
						site.0.load_blocking(Request::get(format!(
							"{BASE_URL}/chapter/{chapter_key}"
						))?)?;
						sleep(5);
					}
					3 | 4 => sleep(5),
					_ => bail!(
						"Toonkotsu wants you to confirm you're human. In Toonkotsu's settings, tap Pass Human Check, open any chapter and pass the check, then reopen this one."
					),
				}
			}
			_ if retries == 2 => {
				bail!("Toonkotsu is limiting reads right now. Try again in a moment.")
			}
			Retry::Resign => retries += 1,
			Retry::Wait(seconds) => {
				retries += 1;
				sleep(seconds);
			}
		}
	};

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
			let url = absolute_url(&page.src);
			let content = match (page.k, page.n) {
				(Some(key), Some(nonce)) => {
					let key = if wrapped {
						signer.unwrap_page_key(&key, &nonce)?
					} else {
						key
					};
					let mut context = PageContext::new();
					context.insert("key".into(), key);
					context.insert("nonce".into(), nonce);
					PageContent::url_context(url, context)
				}
				_ => PageContent::url(url),
			};
			Ok(Page {
				content,
				..Default::default()
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
