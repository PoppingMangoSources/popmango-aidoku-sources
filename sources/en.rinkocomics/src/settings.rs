use aidoku::imports::defaults::defaults_get;

pub fn label_novels() -> bool {
	defaults_get::<bool>("labelNovels").unwrap_or(true)
}

pub fn show_locked_chapters() -> bool {
	defaults_get::<bool>("showLocked").unwrap_or(false)
}
