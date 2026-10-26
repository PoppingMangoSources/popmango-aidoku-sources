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

    def test_silenced_dead_code_fails(self):
        (self.source / 'src').mkdir()
        for attribute in ('#[allow(dead_code)]', '#![allow(unused_imports)]', '#[expect(unused)]'):
            checks.failures.clear()
            (self.source / 'src/lib.rs').write_text(attribute + '\nfn x() {}\n')
            checks.check_rust(self.source, False)
            self.assertTrue(any('silences' in failure for failure in checks.failures), attribute)


if __name__ == '__main__':
    unittest.main()
