use aidoku::imports::defaults::defaults_get;

// The source is rated 17+, so adult novels stay hidden until the reader turns this off.
pub fn hide_adult() -> bool {
	defaults_get::<bool>("hideAdult").unwrap_or(true)
}
