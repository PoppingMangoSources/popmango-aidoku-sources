import importlib.util
import re
from pathlib import Path
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / file)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


usage = load('upstream_usage', 'upstream-usage.py')
site = load('check_site_url', 'check-site-url.py')


class UpstreamUsageTests(unittest.TestCase):
    def test_every_feature_pattern_compiles_and_names_a_feature(self):
        for label, pattern in usage.FEATURES:
            self.assertTrue(label)
            re.compile(pattern)

    def test_usage_cites_the_first_matching_line_once_per_source(self):
        files = {
            'sources/en.a/src/lib.rs': 'fn x() {}\nRequest::send_all(requests)\nRequest::send_all(more)\n',
            'sources/en.a/src/home.rs': 'Request::send_all(again)\n',
            'sources/en.b/src/lib.rs': 'fn y() {}\n',
        }
        hits = usage.usage(files, r'Request::send_all')
        self.assertEqual(list(hits), ['en.a'])
        self.assertIn(hits['en.a'], {'sources/en.a/src/lib.rs:2', 'sources/en.a/src/home.rs:1'})

    def test_features_detect_what_a_source_does(self):
        detected = {label for label, pattern in usage.FEATURES
                    if re.search(pattern, 'send_partial_result(&manga);\nHomeComponentValue::MangaChapterList {')}
        self.assertEqual(detected, {'Partial details while chapters load', 'Chapter list row'})


class SiteUrlTests(unittest.TestCase):
    def test_host_ignores_www_and_case(self):
        self.assertEqual(site.host('https://www.Example.com/a'), site.host('https://example.com'))
        self.assertNotEqual(site.host('https://okutoon.com'), site.host('https://okutoon.net/'))


if __name__ == '__main__':
    unittest.main()
