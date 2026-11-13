import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location("review_checks", ROOT / "scripts/check-sources.py")
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class ReviewRuleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        checks.ROOT = Path(self.temp.name)
        checks.failures.clear()
        checks.notes.clear()

    def template(self, text):
        path = checks.ROOT / "templates/keyoapp/src/imp.rs"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text)
        checks.check_template_review("keyoapp")
        return checks.failures

    def consumer(self, template, name="en.example", filters=None, code=""):
        path = checks.ROOT / "sources" / name
        (path / "src").mkdir(parents=True, exist_ok=True)
        (path / "res").mkdir(exist_ok=True)
        (path / "Cargo.toml").write_text(f'[dependencies]\ntheme = {{path="../../templates/{template}"}}\n')
        (path / "src/lib.rs").write_text(code)
        (path / "res/filters.json").write_text(json.dumps(filters or []))
        return path

    def test_reader_import_requires_minimum_app_on_every_consumer(self):
        source = self.consumer("mangathemesia")
        template = checks.ROOT / "templates/mangathemesia/src"
        template.mkdir(parents=True)
        (template / "markdown.rs").write_text('fn parse() { element.child_nodes(); }\n')
        for version, fails in [("0.7.1", True), ("0.8.2", True), ("0.8.3", False)]:
            checks.failures.clear()
            checks.check_reviewed_consumer(source, {"info": {"minAppVersion": version}})
            self.assertEqual(bool(checks.failures), fails)

    def test_expand_control_only_synopsis_fails(self):
        self.assertTrue(self.template('manga.description = html.select_first("#expand_content").and_then(|el| el.text());\n'))

    def test_short_synopsis_fallbacks_and_comments_pass(self):
        for text in [
            'manga.description = html.select_first("#expand_content, .synopsis p").and_then(|el| el.text());\n',
            'manga.description = html.select_first("#expand_content").or_else(|| html.select_first(".synopsis")).and_then(|el| el.text());\n',
            '// manga.description = html.select_first("#expand_content");\n',
        ]:
            with self.subTest(text=text):
                checks.failures.clear()
                self.assertEqual(self.template(text), [])

    def test_comma_space_only_credits_fail(self):
        self.assertTrue(self.template('pub fn names(text: &str) -> Vec<String> {\n text.split(", ").map(String::from).collect()\n}\n'))

    def test_multiple_credit_delimiters_pass(self):
        self.assertEqual(self.template('fn parse_names(text: &str) -> Vec<String> {\n text.split([\',\', \'&\']).map(str::trim).map(String::from).collect()\n}\n'), [])

    def test_reviewed_include_only_filters_reject_exclusion(self):
        filters = [{"type": "multi-select", "id": "genre", "canExclude": True}]
        for template, name in [("keyoapp", "en.example"), ("mangathemesia", "en.violetscans")]:
            with self.subTest(template=template):
                checks.failures.clear()
                checks.check_reviewed_consumer(self.consumer(template, name, filters), {})
                self.assertTrue(any("include-only" in item for item in checks.failures))

    def test_supported_exclusion_on_another_site_and_disabled_controls_pass(self):
        for template, filters in [("mangathemesia", [{"type": "multi-select", "canExclude": True}]),
                                  ("keyoapp", [{"type": "multi-select", "canExclude": False}])]:
            checks.failures.clear()
            checks.check_reviewed_consumer(self.consumer(template, filters=filters), {})
            self.assertEqual(checks.failures, [])

    def test_rage_does_not_reintroduce_unverified_author_search_or_defaults(self):
        source = self.consumer("mangathemesia", "en.ragescans", [{"type": "text", "id": "author"}])
        checks.check_reviewed_consumer(source, {})
        self.assertTrue(any("author search" in item for item in checks.failures))
        self.assertTrue(any("series_author_selector" in item for item in checks.failures))
        self.assertTrue(any("series_artist_selector" in item for item in checks.failures))
        self.assertTrue(any("viewer override" in item for item in checks.failures))

    def test_scoped_credits_and_viewer_override_pass(self):
        source = self.consumer("mangathemesia", "en.ragescans", code='series_author_selector: r#".tsinfo .imptdt:contains(Author) i"#.into(),\nseries_artist_selector: ".tsinfo .imptdt:contains(Artist) i".into(),\nviewer_override: Some( Viewer::Webtoon ),\n')
        checks.check_reviewed_consumer(source, {})
        self.assertEqual(checks.failures, [])

    def test_a_scoped_selector_cannot_hide_an_unscoped_fallback(self):
        source = self.consumer("mangathemesia", "en.ragescans", code='series_author_selector: ".tsinfo .imptdt:contains(Author) i, span:contains(Author)".into(),\nseries_artist_selector: ".tsinfo .imptdt:contains(Artist) i".into(),\nviewer_override: Some(Viewer::Webtoon),\n')
        checks.check_reviewed_consumer(source, {})
        self.assertTrue(any("metadata rows" in item for item in checks.failures))

    def test_reviewed_icon_regression_and_replacement_note(self):
        source = self.consumer("mangathemesia", "en.madarascans")
        (source / "res/icon.png").write_bytes(b"icon")
        for digest, failures, notes in [
            ("47ab6b5080e05b86b94d3f97350f5e113737fb0330c6a69485390364f655c9b3", 1, 0),
            ("74dc07a11f7bc50097208825f2c9aec07e16c8a91ed0eb6b4dd5e8b52851b011", 0, 0),
            ("replacement artwork", 0, 1),
        ]:
            with self.subTest(digest=digest), patch.object(checks.hashlib, "sha256") as sha:
                checks.failures.clear()
                checks.notes.clear()
                sha.return_value.hexdigest.return_value = digest
                checks.check_reviewed_consumer(source, {})
                self.assertEqual((len(checks.failures), len(checks.notes)), (failures, notes))

    def test_template_only_changes_select_their_consumers(self):
        self.consumer("keyoapp")
        self.consumer("mangathemesia", "en.other")
        with patch.object(checks, "git", return_value="templates/keyoapp/src/helpers.rs\n"):
            self.assertEqual(checks.changed_sources("base"), ["en.example"])


if __name__ == "__main__":
    unittest.main()
