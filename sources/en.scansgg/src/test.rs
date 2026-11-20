use super::*;
use aidoku_test::aidoku_test;

#[aidoku_test]
fn review_chapter_titles_split_at_the_last_group_marker() {
	assert_eq!(
		parse_chapter_title(Some("The End • Part 2 • Night Owls")),
		(Some("The End • Part 2".into()), Some("Night Owls".into()))
	);
	assert_eq!(
		parse_chapter_title(Some("12 • Night Owls")),
		(None, Some("Night Owls".into()))
	);
	assert_eq!(
		parse_chapter_title(Some("Prologue")),
		(Some("Prologue".into()), None)
	);
	assert_eq!(parse_chapter_title(None), (None, None));
}
