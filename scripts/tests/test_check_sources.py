import importlib.util
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SCRIPTS = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('check_sources', SCRIPTS / 'check-sources.py')
checks = importlib.util.module_from_spec(spec)
spec.loader.exec_module(checks)


class CardRuleTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        checks.ROOT = Path(self.temp.name)
        checks.failures.clear()
        self.source = checks.ROOT / 'sources/en.example'
        (self.source / 'res').mkdir(parents=True)
        self.path = self.source / 'lib.rs'

    def test_commercial_controls_and_chapter_exception(self):
        for item in [{"id": "saleFilter", "title": "Availability"}, {"title": "Price"}, {"title": "Availability", "options": ["On Sale"]}]:
            checks.failures.clear()
            checks.check_commercial_control(self.path, item)
            self.assertTrue(checks.failures)
        for item in [{"key": "hidePaidChapters", "title": "Exclude Paid Chapters"}, {"key": "showLocked", "title": "Show Locked Chapters"}, {"isGenre": True, "options": ["Sales"]}]:
            checks.failures.clear()
            checks.check_commercial_control(self.path, item)
            self.assertEqual(checks.failures, [])

    def test_template_lock_requires_all_runtime_dependencies(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\ntheme = { path = "../../templates/theme" }\n')
        template = checks.ROOT / "templates/theme"
        template.mkdir(parents=True)
        (template / "Cargo.toml").write_text('[package]\nname = "theme"\nversion = "0.1.0"\n[dependencies]\ncodec = { package = "base64", version = "0.22" }\nunused = { version = "1", optional = true }\n')
        lock = self.source / "Cargo.lock"
        for dependency, package, fails in [('[]', '', True), ('["base64"]', '', True), ('["base64 0.22.1"]', '[[package]]\nname = "base64"\nversion = "0.22.1"\n', False)]:
            checks.failures.clear()
            lock.write_text('version = 4\n[[package]]\nname = "theme"\nversion = "0.1.0"\ndependencies = ' + dependency + '\n' + package)
            checks.check_template_locks(self.source)
            self.assertEqual(bool(checks.failures), fails)

    def test_reviewed_lock_sync_keeps_upstream_version(self):
        with patch.object(checks, "git", return_value="sources/en.example/Cargo.lock\n"):
            checks.check_version_bump(self.source, 2, "upstream/main", lock_sync=True)
        self.assertEqual(checks.failures, [])

    def test_published_snapshot_needs_no_second_bump(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\niken = { path = "../../templates/iken" }\n')
        with patch.object(checks, "git", return_value="") as git:
            checks.check_version_bump(self.source, 3, "origin/main")
        self.assertEqual(checks.failures, [])
        self.assertIn("templates/iken", git.call_args.args)

    def test_changed_template_still_requires_bump(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\niken = { path = "../../templates/iken" }\n')
        with patch.object(checks, "git", side_effect=["templates/iken/src/imp.rs\n", '{"info":{"version":3}}']):
            checks.check_version_bump(self.source, 3, "origin/main")
        self.assertTrue(any("not bumped" in failure for failure in checks.failures))

    def test_fork_regressions_need_no_publication_bump(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\niken = { path = "../../templates/iken" }\n')
        with patch.object(checks, "git", return_value="sources/en.example/src/test.rs\ntemplates/iken/src/source_test.rs\n") as git:
            checks.check_version_bump(self.source, 3, "origin/main")
        self.assertEqual(checks.failures, [])
        self.assertEqual(git.call_count, 1)

    def test_regression_beside_runtime_change_still_needs_bump(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\niken = { path = "../../templates/iken" }\n')
        with patch.object(checks, "git", side_effect=["templates/iken/src/source_test.rs\ntemplates/iken/src/imp.rs\n", '{"info":{"version":3}}']):
            checks.check_version_bump(self.source, 3, "origin/main")
        self.assertTrue(any("not bumped" in failure for failure in checks.failures))

    def cards(self, text):
        checks.check_cards(self.path, text)
        return list(checks.failures)

    def test_row_subtitle_fails(self):
        text = 'fn row(m: Manga) -> Link {\n\tLink {\n\t\tsubtitle: Some(label),\n\t\t..m.into()\n\t}\n}\n'
        self.assertTrue(any('subtitle' in failure for failure in self.cards(text)))

    def test_row_without_subtitle_passes(self):
        text = 'let a: Link = manga.into();\nLink { title, subtitle: None, image_url, value }\n'
        self.assertEqual(self.cards(text), [])

    def test_section_subtitle_is_not_a_row(self):
        text = 'HomeComponent {\n\ttitle: Some(name),\n\tsubtitle: Some("Climbing fast".into()),\n\tvalue,\n}\n'
        self.assertEqual(self.cards(text), [])

    def test_full_details_in_search_fail(self):
        text = (
            'impl Source for X {\n\tfn get_search_manga_list(&self) -> Result<MangaPageResult> {\n'
            '\t\tlet manga = self.get_manga_update(m, true, false)?;\n\t}\n\n'
            '\tfn get_manga_update(&self) -> Result<Manga> {\n\t}\n}\n'
        )
        self.assertTrue(any('full details' in failure for failure in self.cards(text)))

    def test_justified_silence_passes(self):
        (self.source / 'src').mkdir()
        (self.source / 'src/lib.rs').write_text('// Tells the untagged variants apart.\n#[allow(dead_code)]\nheader: String,\n')
        checks.check_rust(self.source, False)
        self.assertEqual(checks.failures, [])

    def test_redundant_tag_style_fails(self):
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "multi-select", "id": "genre", "isGenre": true, "usesTagStyle": true, "options": []}\n]\n')
        checks.check_filters_json(path)
        self.assertTrue(any('usesTagStyle' in failure for failure in checks.failures))

    def test_explicitly_disabled_exclusion_passes(self):
        checks.notes.clear()
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "multi-select", "id": "status", "canExclude": false, "options": []}\n]\n')
        checks.check_filters_json(path)
        self.assertEqual(checks.failures, [])
        self.assertEqual(checks.notes, [])

    def test_unstated_exclusion_is_noted(self):
        checks.notes.clear()
        path = self.source / 'res/filters.json'
        path.write_text(
            '[\n\t{"type": "multi-select", "id": "type", "options": []},\n'
            '\t{"type": "multi-select", "id": "genres", "isGenre": true, "canExclude": true, "options": []}\n]\n'
        )
        checks.check_filters_json(path)
        self.assertEqual(checks.failures, [])
        self.assertEqual(len(checks.notes), 1)
        self.assertIn("'type' leaves canExclude unstated", checks.notes[0])

    def test_reviewed_languages_preserve_an_explicit_empty_selection(self):
        self.source = self.source.rename(self.source.with_name("multi.mangaball"))
        (self.source / "src").mkdir()
        (self.source / "Cargo.toml").write_text("[dependencies]\n")
        settings = self.source / "src/settings.rs"
        for expression, fails in [
            ('defaults_get::<Vec<String>>("languages").filter(|languages| !languages.is_empty()).unwrap_or_else(default)', True),
            ('defaults_get::<Vec<String>>("languages").unwrap_or_else(default)', False),
        ]:
            checks.failures.clear()
            settings.write_text(expression)
            checks.check_reviewed_consumer(self.source, {})
            self.assertEqual(bool(checks.failures), fails)

    def test_sort_after_other_filters_is_noted(self):
        checks.notes.clear()
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "text", "id": "author"},\n\t{"type": "sort", "id": "sort", "options": []}\n]\n')
        checks.check_filters_json(path)
        self.assertTrue(any('sort filter is not first' in n for n in checks.notes))
        checks.notes.clear()
        path.write_text('[\n\t{"type": "sort", "id": "sort", "options": []},\n\t{"type": "text", "id": "author"}\n]\n')
        checks.check_filters_json(path)
        self.assertEqual(checks.notes, [])

    def test_adult_shown_by_default_on_rating_one_fails(self):
        path = self.source / 'res/settings.json'
        for key, default in (('hideAdult', 'false'), ('showNsfw', 'true')):
            checks.failures.clear()
            path.write_text(f'[\n\t{{"type": "group", "items": [{{"type": "switch", "key": "{key}", "default": {default}}}]}}\n]\n')
            checks.check_settings_json(path, 1)
            self.assertTrue(any('shows adult titles by default' in f for f in checks.failures), key)

    def test_adult_hidden_by_default_or_rating_two_passes(self):
        path = self.source / 'res/settings.json'
        path.write_text('[\n\t{"type": "switch", "key": "hideAdult", "default": true}\n]\n')
        checks.check_settings_json(path, 1)
        path.write_text('[\n\t{"type": "switch", "key": "hideAdult", "default": false}\n]\n')
        checks.check_settings_json(path, 2)
        self.assertEqual(checks.failures, [])

    def test_hentai_genre_on_a_safe_source_fails(self):
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "multi-select", "id": "genre", "isGenre": true, "options": ["Action", "Hentai"]}\n]\n')
        checks.check_filters_json(path, 0)
        self.assertTrue(any('Hentai genre' in f for f in checks.failures))

    def test_hentai_genre_on_a_rated_source_or_adult_genres_pass(self):
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "multi-select", "id": "genre", "isGenre": true, "options": ["Action", "Hentai"]}\n]\n')
        checks.check_filters_json(path, 1)
        path.write_text('[\n\t{"type": "multi-select", "id": "genre", "isGenre": true, "options": ["Adult", "Smut"]}\n]\n')
        checks.check_filters_json(path, 0)
        self.assertEqual(checks.failures, [])

    def test_unsized_image_scroller_is_noted(self):
        checks.notes.clear()
        unsized = 'HomeComponentValue::ImageScroller {\n\tlinks,\n\twidth: None,\n\theight: None,\n}\n'
        checks.check_cards(self.path, unsized)
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_cards(self.path, unsized.replace('width: None', 'width: Some(300)'))
        self.assertEqual(checks.notes, [])

    def test_bracket_in_attribute_selector_is_noted(self):
        checks.notes.clear()
        checks.check_cards(self.path, 'html.select_first("a.slide [class*=\'aspect-[2/1]\']")\n')
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_cards(self.path, 'html.select("a[href*=\'/series/\'], img[alt$=coins]")\n')
        self.assertEqual(checks.notes, [])

    def api_use(self, text):
        checks.failures.clear()
        checks.notes.clear()
        checks.check_api_use(self.path, text)
        return checks.failures + checks.notes

    def test_api_use_rules_fail(self):
        cases = {
            'bare key': 'query.push("sort", None);\n',
            'TryFrom': 'impl From<Entry> for Manga {\n\tfn from(e: Entry) -> Self {\n\t\tlet key = e.key().unwrap_or_default();\n\t}\n}\n',
            'en_US_POSIX': 'let date = parse_date(text.trim(), "MMM d, yyyy");\n',
            'json_owned': 'let body = Request::get(url)?.string()?;\nlet list: List = serde_json::from_str(&body)?;\n',
            'uses_tag_style': 'MultiSelectFilter {\n\tid: "genre".into(),\n\tis_genre: true,\n\t..Default::default()\n}\n',
            'included for emptiness': 'if !included.is_empty() {\n}\n',
            'strip_prefix': 'let rest = &url[BASE_URL.len()..];\n',
            'remaps a request': 'Request::get(url)\n\t.map_err(|_| error!("bad"))?;\n',
            'escapes a whole chapter': 'content: PageContent::text(text.escape_markdown()),\n',
        }
        for rule, text in cases.items():
            self.assertTrue(any(rule in message for message in self.api_use(text)), rule)

    def test_api_use_rules_pass(self):
        text = (
            'if let Some(sort) = sort {\n\tquery.push("sort", Some(sort));\n}\n'
            'impl TryFrom<Entry> for Manga {\n\tfn try_from(e: Entry) -> Result<Self> {\n\t\tlet key = e.key().ok_or_else(|| error!("no key"))?;\n\t}\n}\n'
            'impl From<Entry> for Manga {\n\tfn from(e: Entry) -> Self {\n\t\tlet description = e.text.unwrap_or_default();\n\t}\n}\n'
            'let date = parse_date(text, "yyyy-MM-dd");\n'
            'let date = parse_date_with_options(text, "MMM d, yyyy", "en_US_POSIX", "current");\n'
            'let list: List = Request::get(url)?.send()?.json_owned()?;\n'
            'let data: Next = serde_json::from_str(&script.data().unwrap_or_default())?;\n'
            'MultiSelectFilter {\n\tis_genre: true,\n\tuses_tag_style: true,\n\t..Default::default()\n}\n'
            'let rest = url.strip_prefix(BASE_URL)?;\n'
            'Request::get(url)?.send()?;\n'
            'let text = lines.map(|line| line.escape_markdown()).collect::<Vec<_>>().join("\\n\\n");\n'
            'content: PageContent::text(text),\n'
        )
        self.assertEqual(self.api_use(text), [])

    def test_locked_shown_by_default_fails_without_login(self):
        path = self.source / 'res/settings.json'
        path.write_text('[\n\t{"type": "switch", "key": "showLocked", "default": true}\n]\n')
        checks.check_settings_json(path, 0)
        self.assertTrue(any('locked chapters' in f for f in checks.failures))
        checks.failures.clear()
        path.write_text('[\n\t{"type": "login", "key": "login"},\n\t{"type": "switch", "key": "showLocked", "default": true}\n]\n')
        checks.check_settings_json(path, 0)
        path.write_text('[\n\t{"type": "switch", "key": "showLocked", "default": false}\n]\n')
        checks.check_settings_json(path, 0)
        self.assertEqual(checks.failures, [])

    def test_test_without_assertion_fails(self):
        (self.source / 'src').mkdir()
        (self.source / 'src/test.rs').write_text('#[aidoku_test]\nfn found_type_is_used() {\n\tlet _ = Found { hits: Vec::new() }.hits;\n}\n')
        checks.check_tests(self.source)
        self.assertTrue(any('asserts nothing' in failure for failure in checks.failures))

    def test_release_profile_required(self):
        path = self.source / 'Cargo.toml'
        path.write_text('[package]\nname = "x"\n\n[profile.release]\npanic = "abort"\n')
        checks.check_cargo(path)
        self.assertTrue(any('lto = true' in failure for failure in checks.failures))

    def test_stale_aidoku_lock_fails(self):
        path = self.source / 'Cargo.lock'
        path.write_text('[[package]]\nname = "aidoku"\nsource = "git+https://github.com/Aidoku/aidoku-rs.git#1a6bb691dd67c7151fc76fc852fb5a364d325f72"\n')
        checks.check_lock(path)
        self.assertTrue(any('locks aidoku-rs 1a6bb691' in failure for failure in checks.failures))

    def test_current_aidoku_lock_passes(self):
        path = self.source / 'Cargo.lock'
        path.write_text('[[package]]\nname = "aidoku"\nsource = "git+https://github.com/Aidoku/aidoku-rs.git#ae6485e3c14ecd1fd5497d170eae884c68d04a87"\n')
        checks.failures.clear()
        checks.check_lock(path)
        self.assertEqual(checks.failures, [])

    def test_unused_json_test_dependency_is_noted(self):
        (self.source / 'src').mkdir()
        (self.source / 'Cargo.toml').write_text('[dev-dependencies]\nserde_json = "1"\n')
        checks.notes.clear()
        checks.check_test_dependencies(self.source)
        self.assertTrue(any('serde_json dev dependency' in note for note in checks.notes))

    def test_used_json_test_dependency_passes(self):
        (self.source / 'src').mkdir()
        (self.source / 'src/test.rs').write_text('let item = serde_json::from_str("{}");\n')
        (self.source / 'Cargo.toml').write_text('[dev-dependencies]\nserde_json = "1"\n')
        checks.notes.clear()
        checks.check_test_dependencies(self.source)
        self.assertEqual(checks.notes, [])

    def naming(self, text, name='lib.rs'):
        (self.source / 'src').mkdir(exist_ok=True)
        (self.source / 'src' / name).write_text(text)
        checks.notes.clear()
        checks.check_naming(self.source)
        checks.check_rust(self.source, False)
        return list(checks.failures), list(checks.notes)

    def test_endpoint_constant_fails(self):
        failures, _ = self.naming(
            'const BASE_URL: &str = "https://x.moe";\nconst SEARCH_URL: &str = "https://x.moe/search";\n'
            'fn url(q: &str) -> String { format!("{SEARCH_URL}?{q}") }\n'
        )
        self.assertTrue(any('SEARCH_URL is BASE_URL plus a path' in f for f in failures))

    def test_api_base_and_referer_pass(self):
        failures, _ = self.naming(
            'const BASE_URL: &str = "https://x.moe";\nconst API_URL: &str = "https://x.moe/api";\n'
            'const QUERY_URL: &str = "https://x.moe/graphql";\nconst REFERER: &str = "https://x.moe/";\n'
            'fn a() { format!("{API_URL}/search?{q}"); Request::post(QUERY_URL); referer(REFERER); }\n'
        )
        self.assertEqual(failures, [])

    def test_single_use_query_constant_fails(self):
        failures, _ = self.naming('const NUM_TYPOS: &str = "4,3";\nfn a() { query.push("num_typos", Some(NUM_TYPOS)); }\n')
        self.assertTrue(any('NUM_TYPOS is sent once' in f for f in failures))

    def test_shared_query_constant_passes(self):
        failures, _ = self.naming(
            'const FIELDS: &str = "id";\nfn a() { query.push("fields", Some(FIELDS)); }\n'
            'fn b() { query.push("include", Some(FIELDS)); }\n'
        )
        self.assertEqual(failures, [])

    def test_page_size_name_is_noted(self):
        _, notes = self.naming('const FEED_LIMIT: i32 = 24;\nfn a() { query.push("limit", Some(&FEED_LIMIT.to_string())); }\n')
        self.assertTrue(any('FEED_LIMIT is sent as a page size' in n for n in notes))
        _, notes = self.naming('const PAGE_SIZE: i32 = 24;\nfn a() { query.push("limit", Some(&PAGE_SIZE.to_string())); }\n')
        self.assertEqual(notes, [])

    def test_own_field_wrapper_is_noted(self):
        text = (
            'impl Info {\n\tpub fn is_novel(&self) -> bool {\n\t\tis_novel(self.medium.as_deref())\n\t}\n}\n'
            'impl Source for X {\n\tfn get_manga_list(&self, id: &str) -> R {\n\t\thome::listing(id)\n\t}\n}\n'
        )
        _, notes = self.naming(text)
        self.assertTrue(any('fn is_novel only passes its own fields' in n for n in notes))
        self.assertFalse(any('get_manga_list' in n for n in notes))

    def test_imported_helper_used_once_is_noted(self):
        (self.source / 'src').mkdir(exist_ok=True)
        (self.source / 'src/api.rs').write_text('pub fn page_url(id: &str) -> String {\n\tapi_url("page", id)\n}\n')
        _, notes = self.naming('use api::{get, page_url};\nfn a(k: &str) { get(&page_url(k)); }\n')
        self.assertTrue(any('fn page_url only calls api_url and is used once' in n for n in notes))

    def test_full_path_call_is_noted(self):
        _, notes = self.naming('fn new() -> Self {\n\tSelf { seed: aidoku::imports::std::current_date() }\n}\n')
        self.assertTrue(any('by its full path' in n for n in notes))
        _, notes = self.naming('use aidoku::imports::std::current_date;\nfn d(e: aidoku::imports::html::Element) {}\n')
        self.assertEqual(notes, [])

    def test_silenced_dead_code_fails(self):
        (self.source / 'src').mkdir()
        for attribute in ('#[allow(dead_code)]', '#![allow(unused_imports)]', '#[expect(unused)]'):
            checks.failures.clear()
            (self.source / 'src/lib.rs').write_text(attribute + '\nfn x() {}\n')
            checks.check_rust(self.source, False)
            self.assertTrue(any('silences' in failure for failure in checks.failures), attribute)




class KeyoappHttpTests(unittest.TestCase):
    def test_only_checked_fetches_are_allowed_in_keyoapp_parsers(self):
        previous_root = checks.ROOT
        with tempfile.TemporaryDirectory() as directory:
            try:
                checks.ROOT = Path(directory)
                folder = checks.ROOT / "templates/keyoapp/src"
                folder.mkdir(parents=True)
                for name in ("imp.rs", "v2.rs"):
                    with self.subTest(name=name):
                        path = folder / name
                        path.write_text('let html = Request::get(url)?.html()?;\n')
                        checks.failures.clear()
                        checks.check_template_http("keyoapp")
                        self.assertEqual(len(checks.failures), 1)
                        self.assertIn("bypasses keyoapp HTTP status checks", checks.failures[0])
                        path.write_text('let html = fetch(url)?.get_html()?;\n')
                        checks.failures.clear()
                        checks.check_template_http("keyoapp")
                        self.assertEqual(checks.failures, [])
                (folder / "helpers.rs").write_text('Request::get(url)?.send()?;\n')
                checks.check_template_http("keyoapp")
                self.assertEqual(checks.failures, [])
            finally:
                checks.ROOT = previous_root


class MangaThemesiaHttpTests(unittest.TestCase):
    def test_parsed_requests_are_checked_but_image_factories_remain_valid(self):
        previous_root = checks.ROOT
        with tempfile.TemporaryDirectory() as directory:
            try:
                checks.ROOT = Path(directory)
                folder = checks.ROOT / "templates/mangathemesia/src"
                folder.mkdir(parents=True)
                for name in ("imp.rs", "child_theme.rs"):
                    for method in ("html", "string", "json_owned"):
                        with self.subTest(name=name, method=method):
                            path = folder / name
                            path.write_text(f'let body = Request::get(url)?.{method}()?;\n')
                            checks.failures.clear()
                            checks.check_template_http("mangathemesia")
                            self.assertEqual(len(checks.failures), 1)
                            path.write_text('let body = fetch(url)?.get_html()?;\n')
                            checks.failures.clear()
                            checks.check_template_http("mangathemesia")
                            self.assertEqual(checks.failures, [])
                (folder / "imp.rs").write_text('fn image_request() { Ok(Request::get(url)?.header("Accept", "image/*")) }\nfn text() { element.html() }\n')
                checks.check_template_http("mangathemesia")
                self.assertEqual(checks.failures, [])
            finally:
                checks.ROOT = previous_root


if __name__ == '__main__':
    unittest.main()

class IkenHttpTests(unittest.TestCase):
    def test_home_requires_checked_response_but_image_factory_does_not(self):
        previous_root = checks.ROOT
        with tempfile.TemporaryDirectory() as directory:
            try:
                checks.ROOT = Path(directory)
                folder = checks.ROOT / "templates/iken/src"
                folder.mkdir(parents=True)
                path = folder / "home.rs"
                path.write_text('let html = Request::get(url)?.html()?;\n')
                checks.failures.clear()
                checks.check_template_http("iken")
                self.assertEqual(len(checks.failures), 1)
                path.write_text('let html = check_response(Request::get(url)?.send()?, url)?.get_html()?;\n')
                (folder / "imp.rs").write_text('fn image() { Ok(Request::get(url)?.header("Referer", base)) }\n')
                checks.failures.clear()
                checks.check_template_http("iken")
                self.assertEqual(checks.failures, [])
            finally:
                checks.ROOT = previous_root
