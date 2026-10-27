import importlib.util
from pathlib import Path
import tempfile
import unittest

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
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "multi-select", "id": "status", "canExclude": false, "options": []}\n]\n')
        checks.check_filters_json(path)
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


if __name__ == '__main__':
    unittest.main()
