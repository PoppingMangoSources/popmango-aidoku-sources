use aidoku::{
	alloc::{String, Vec, format},
	imports::defaults::{defaults_get, defaults_get_map},
};

pub const VERIFY_KEY: &str = "verify";

pub fn show_locked_chapters() -> bool {
	defaults_get::<bool>("showLocked").unwrap_or(false)
}

pub fn verify_cookie_header() -> Option<String> {
	let cookies = defaults_get_map(VERIFY_KEY)?;
	let pairs: Vec<String> = cookies
		.into_iter()
		.filter(|(k, v)| !k.is_empty() && !v.is_empty())
		.map(|(k, v)| format!("{k}={v}"))
		.collect();
	(!pairs.is_empty()).then(|| pairs.join("; "))
}
