use aidoku::alloc::{String, Vec, format, string::ToString};

pub const SEED: &str = "di183bafh964gce5072";

const POSITION_MULTIPLIER: u32 = 795_580_989;
const SEED_MULTIPLIER: u32 = 2_048_675_417;
const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
const DECLARATION_WINDOW: usize = 600;
const MODULE_WINDOW: usize = 8 * 1024;

pub fn decode(token: &str, index: usize, seed: &str) -> Option<String> {
	let mut state = initial_state(index, seed)?;
	let bytes: Vec<u8> = unscramble(token)
		.into_iter()
		.map(|byte| {
			state = xorshift(state);
			byte ^ (state as u8)
		})
		.collect();
	String::from_utf8(bytes).ok()
}

fn initial_state(index: usize, seed: &str) -> Option<u32> {
	let position = u32::try_from(index).ok()?.wrapping_add(1);
	let length = u32::try_from(seed.len()).ok()?;
	let mut state = position.wrapping_mul(POSITION_MULTIPLIER) ^ length;
	for byte in seed.bytes() {
		state = (state ^ u32::from(byte)).wrapping_mul(SEED_MULTIPLIER);
		state ^= state >> 15;
	}
	Some(if state == 0 { 1 } else { state })
}

fn xorshift(mut state: u32) -> u32 {
	state ^= state << 13;
	state ^= state >> 17;
	state ^= state << 5;
	state
}

fn unscramble(token: &str) -> Vec<u8> {
	let mut bytes = Vec::with_capacity(token.len() * 3 / 4);
	let mut bits = 0_u32;
	let mut count = 0_u32;
	for ch in token.bytes() {
		let Some(value) = ALPHABET
			.iter()
			.position(|&letter| letter == ch)
			.and_then(|v| u32::try_from(v).ok())
		else {
			continue;
		};
		bits = ((bits << 6) | value) & 0xFFFF;
		count += 6;
		if count >= 8 {
			count -= 8;
			bytes.push((bits >> count) as u8);
		}
	}
	bytes
}

pub fn seed_from_chunk(chunk: &str, module: u64) -> Option<String> {
	let marker = format!("{module}:function(");
	let start = chunk
		.match_indices(&marker)
		.find(|&(at, _)| at == 0 || matches!(chunk.as_bytes()[at - 1], b'{' | b','))?
		.0;
	let body = prefix(&chunk[start..], MODULE_WINDOW);
	if !body.contains(&POSITION_MULTIPLIER.to_string())
		|| !body.contains(&SEED_MULTIPLIER.to_string())
	{
		return None;
	}
	let head = prefix(body, DECLARATION_WINDOW);
	let decoder = ["new TextEncoder", "new TextDecoder"]
		.iter()
		.filter_map(|name| head.find(name))
		.min()?;
	head[..decoder]
		.split('"')
		.skip(1)
		.step_by(2)
		.find(|literal| plausible_seed(literal))
		.map(String::from)
}

fn plausible_seed(literal: &str) -> bool {
	(8..=64).contains(&literal.len()) && literal.bytes().all(|b| b.is_ascii_alphanumeric())
}

fn prefix(text: &str, max: usize) -> &str {
	let mut end = max.min(text.len());
	while !text.is_char_boundary(end) {
		end -= 1;
	}
	&text[..end]
}

#[cfg(test)]
pub fn scramble(address: &str, index: usize, seed: &str) -> String {
	let mut state = initial_state(index, seed).expect("position fits");
	let bytes: Vec<u8> = address
		.bytes()
		.map(|byte| {
			state = xorshift(state);
			byte ^ (state as u8)
		})
		.collect();
	let mut token = String::new();
	let (mut bits, mut count) = (0_u32, 0_u32);
	for byte in bytes {
		bits = (bits << 8) | u32::from(byte);
		count += 8;
		while count >= 6 {
			count -= 6;
			token.push(char::from(ALPHABET[((bits >> count) & 63) as usize]));
		}
	}
	if count > 0 {
		token.push(char::from(ALPHABET[((bits << (6 - count)) & 63) as usize]));
	}
	token
}
