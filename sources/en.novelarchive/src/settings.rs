use aidoku::imports::defaults::defaults_get;

// The source is rated 1, so adult titles stay hidden until the reader turns the switch off.
pub fn hide_adult() -> bool {
	defaults_get::<bool>("hideAdult").unwrap_or(true)
}
