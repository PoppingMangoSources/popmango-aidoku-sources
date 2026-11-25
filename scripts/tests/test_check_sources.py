import importlib.util
import json
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
        for item in [{"key": "hidePaidChapters", "title": "Exclude Paid Chapters"}, {"key": "showLocked", "title": "Show Locked Chapters"}, {"isGenre": True, "options": ["Sales"]}, {"usesTagStyle": True, "options": ["Salesman"]}]:
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

    def test_breaking_change_version_is_kept(self):
        (self.source / "Cargo.toml").write_text('[dependencies]\n')
        before = '{"info":{"version":3},"config":{"breakingChangeVersion":3}}'
        for config, fails in [({}, True), ({"breakingChangeVersion": 3}, False)]:
            checks.failures.clear()
            (self.source / "res/source.json").write_text(checks.json.dumps({"info": {"version": 4}, "config": config}))
            with patch.object(checks, "git", side_effect=["sources/en.example/res/source.json\n", before]):
                checks.check_version_bump(self.source, 4, "upstream/main")
            self.assertEqual(any("breakingChangeVersion" in f for f in checks.failures), fails)

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

    def test_webnex_preset_layout_and_sendable_first_choice(self):
        path = checks.ROOT / 'sources/en.webnex/res/filters.json'
        path.parent.mkdir(parents=True)
        good = [{"type": "text", "id": "minChapters"}] + [
            {"type": "select", "id": key, "default": "", "usesTagStyle": True,
             "options": [label], "ids": [value]}
            for key, label, value in [("chapters", "Readable Now", "1"), ("released", "This Year", "this-year")]]
        invalid = []
        for change in [{"default": "1"}, {"usesTagStyle": False}, {"options": ["Any", "Readable Now"]}, {"ids": ["", "1"]}]:
            invalid.append([good[0], {**good[1], **change}, good[2]])
        invalid.extend([[good[1], good[0], good[2]], [good[0], good[2]]])
        for data, fails in [(good, False)] + [(data, True) for data in invalid]:
            checks.failures.clear()
            path.write_text(json.dumps(data, indent='\t') + '\n')
            checks.check_filters_json(path)
            self.assertEqual(any('WebNex presets' in failure for failure in checks.failures), fails)
        for control, fails in [({"type": "text", "id": "tags"}, True), ({"type": "multi-select", "id": "theme", "canExclude": True}, False)]:
            checks.failures.clear()
            path.write_text(json.dumps(good + [control], indent='\t') + '\n')
            checks.check_filters_json(path)
            self.assertEqual(any('typed Tags search' in failure for failure in checks.failures), fails)

    def test_other_sources_can_keep_neutral_select_choices(self):
        path = self.source / 'res/filters.json'
        path.write_text(json.dumps([{"type": "select", "id": "chapters", "options": ["Any", "Readable"]}], indent='\t') + '\n')
        checks.check_filters_json(path)
        self.assertEqual(checks.failures, [])

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

    def test_reviewed_markdown_descriptions_are_not_escaped(self):
        self.source = self.source.rename(self.source.with_name("multi.mangaball"))
        (self.source / "src").mkdir()
        (self.source / "Cargo.toml").write_text("[dependencies]\n")
        library = self.source / "src/lib.rs"
        for expression, fails in [
            ('details.description.map(|text| text.escape_markdown())', True),
            ('manga.copy_from(details)', False),
        ]:
            checks.failures.clear()
            library.write_text(expression)
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

    def test_long_listing_name_is_noted(self):
        path = self.source / 'res/source.json'
        body = '{\n\t"info": {\n\t\t"id": "en.example",\n\t\t"minAppVersion": "0.7.1"\n\t},\n\t"listings": [\n\t\t{"id": "a", "name": "%s"},\n\t\t{"id": "b", "name": "%s"}\n\t]\n}\n'
        checks.notes.clear()
        path.write_text(body % ('Latest Chapter Updates', 'Recently Added Titles'))
        checks.check_source_json(path, 'en.example')
        self.assertEqual(sum('ends like a home heading' in n for n in checks.notes), 2)
        checks.notes.clear()
        path.write_text(body % ('Latest Updates', 'Recently Added'))
        checks.check_source_json(path, 'en.example')
        self.assertEqual(checks.notes, [])

    def test_urls_repeating_url_fails(self):
        path = self.source / 'res/source.json'
        body = '{\n\t"info": {\n\t\t"id": "en.example",\n\t\t"url": "https://a.example",\n\t\t%s"minAppVersion": "0.7.1"\n\t}\n}\n'
        checks.failures.clear()
        path.write_text(body % '"urls": ["https://a.example"],\n\t\t')
        checks.check_source_json(path, 'en.example')
        self.assertTrue(any('urls only repeats url' in f for f in checks.failures))
        for urls in ('', '"urls": ["https://a.example", "https://b.example"],\n\t\t'):
            checks.failures.clear()
            path.write_text(body % urls)
            checks.check_source_json(path, 'en.example')
            self.assertEqual(checks.failures, [])

    def test_genre_flag_after_options_is_noted(self):
        checks.notes.clear()
        path = self.source / 'res/filters.json'
        path.write_text('[\n\t{"type": "select", "id": "genre", "options": ["a"], "isGenre": true}\n]\n')
        checks.check_filters_json(path)
        self.assertTrue(any('isGenre after its options' in n for n in checks.notes))
        checks.notes.clear()
        path.write_text('[\n\t{"type": "select", "id": "genre", "isGenre": true, "options": ["a"]}\n]\n')
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

    def test_banner_wider_than_a_phone_fails(self):
        checks.failures.clear()
        checks.check_cards(self.path, 'Params {\n\tbanner_width: Some(1920),\n\tbanner_height: Some(1080),\n}\n')
        self.assertEqual(len(checks.failures), 1)
        checks.failures.clear()
        checks.check_cards(self.path, 'ImageScroller {\n\tlinks,\n\twidth: Some(340),\n\theight: Some(124),\n}\n')
        self.assertEqual(checks.failures, [])

    def test_trimmed_text_is_noted(self):
        checks.notes.clear()
        checks.check_cards(self.path, '.and_then(|title| title.text())\n\t.filter(|title| !title.trim().is_empty())\n')
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_cards(self.path, '.attr("alt")\n\t.filter(|title| !title.trim().is_empty())\n')
        self.assertEqual(checks.notes, [])

    def test_optional_rewrap_is_noted(self):
        checks.notes.clear()
        checks.check_cards(self.path, 'if let Some(query) = query {\n\tqs.push("search", Some(&query));\n}\n')
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_cards(self.path, 'if query.is_some() {\n\tqs.push("search", query.as_deref());\n}\n')
        checks.check_cards(self.path, 'if let Some(query) = query {\n\tqs.push("search", Some(&query.to_lowercase()));\n}\n')
        self.assertEqual(checks.notes, [])

    def test_stable_sort_is_noted(self):
        checks.notes.clear()
        checks.check_cards(self.path, 'let mut tags = names();\ntags.sort();\n')
        checks.check_cards(self.path, 'let mut rows = list();\nrows.sort_by_key(|row| row.age);\n')
        self.assertEqual(len(checks.notes), 2)
        checks.notes.clear()
        checks.check_cards(self.path, 'let mut tags = names();\ntags.sort_unstable();\n')
        checks.check_cards(self.path, '// Stable, so equal ages keep the page order.\nrows.sort_by_key(|row| row.age);\n')
        self.assertEqual(checks.notes, [])

    def test_query_appended_to_a_url_variable_is_noted(self):
        checks.notes.clear()
        checks.check_cards(self.path, 'format!("{CDN_URL}/{path}?w={quality}")\n')
        checks.check_cards(self.path, 'format!("{url}?page={page}")\n')
        self.assertEqual(len(checks.notes), 2)
        checks.notes.clear()
        checks.check_cards(self.path, 'format!("{BASE_URL}/search?{query}")\n')
        checks.check_cards(self.path, 'format!("{CDN_URL}/{path}{separator}w={quality}")\n')
        checks.check_cards(self.path, 'format!("{}/{}?{query}", params.base_url, params.catalog)\n')
        checks.check_cards(self.path, '// Both paths are fixed, so the query starts here.\nlet url = format!("{API_URL}/{path}?{query}");\n')
        self.assertEqual(checks.notes, [])

    def test_utc_timestamp_parsed_in_local_time_is_noted(self):
        checks.notes.clear()
        checks.check_api_use(self.path, ".and_then(|date| parse_date(date, \"yyyy-MM-dd'T'HH:mm:ss.SSS'Z'\"))\n")
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_api_use(self.path, "parse_date_with_options(date, \"yyyy-MM-dd'T'HH:mm:ss.SSS'Z'\", \"en_US_POSIX\", \"UTC\")\n")
        checks.check_api_use(self.path, 'parse_date(date, "dd/MM/yyyy")\n')
        self.assertEqual(checks.notes, [])

    def test_iso_timestamp_reshaped_before_parsing_is_noted(self):
        checks.notes.clear()
        checks.check_api_use(self.path, ".and_then(|date| parse_date_with_options(date.replace('T', \" \"), \"yyyy-MM-dd HH:mm:ss\", \"en_US_POSIX\", \"UTC\"))\n")
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_api_use(self.path, "parse_date_with_options(date, \"yyyy-MM-dd'T'HH:mm:ss.SSSXXX\", \"en_US_POSIX\", \"UTC\")\n")
        checks.check_api_use(self.path, "let name = name.replace('T', \" \");\n")
        self.assertEqual(checks.notes, [])

    def test_deep_link_scheme_split_is_noted(self):
        checks.notes.clear()
        checks.check_api_use(self.path, 'fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {\n\tlet Some(path) = url.split_once("://").map(|(_, rest)| rest) else {\n')
        self.assertEqual(len(checks.notes), 1)
        checks.notes.clear()
        checks.check_api_use(self.path, 'fn handle_deep_link(&self, url: String) -> Result<Option<DeepLinkResult>> {\n\tlet Some(path) = url.strip_prefix(BASE_URL) else {\n')
        checks.check_api_use(self.path, 'fn host(url: &str) -> Option<&str> {\n\turl.split_once("://")\n}\n')
        self.assertEqual(checks.notes, [])

    def test_top_level_items_need_blank_lines(self):
        path = checks.ROOT / 'sources/es.eternalmangas/src/models.rs'
        checks.failures.clear()
        checks.check_item_spacing(path, 'mod helpers;\nconst URL: &str = "x";\nstruct A;\n#[derive(Deserialize)]\npub struct B {\n}\n#[derive(Deserialize)]\npub struct C {\n}\n')
        self.assertEqual(len(checks.failures), 3)
        checks.failures.clear()
        checks.check_item_spacing(path, 'mod helpers;\nmod home;\n\nconst URL: &str = "x";\n\nstruct A;\n\n#[derive(Deserialize)]\npub struct B {\n}\n')
        self.assertEqual(checks.failures, [])

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
            'statements to eval_async': 'web_view.eval_async(&format!("return await window.state.result;"))?;\n',
            'defaults Listing.name with unwrap_or_default': 'Listing {\n\tid,\n\tname: title.clone().unwrap_or_default(),\n\t..Default::default()\n};\n',
            'defaults Manga.title with unwrap_or_default': 'Manga {\n\tkey,\n\ttitle: heading.text().unwrap_or_default(),\n\t..Default::default()\n};\n',
            'defaults Link.title with unwrap_or_default': 'Link {\n\ttitle: name.unwrap_or_default(),\n\t..Default::default()\n};\n',
            'defaults Chapter.key with unwrap_or_default': 'Chapter {\n\tkey: id.unwrap_or_default(),\n\t..Default::default()\n};\n',
        }
        for rule, text in cases.items():
            self.assertTrue(any(rule in message for message in self.api_use(text)), rule)

    def test_api_use_rules_pass(self):
        text = (
            'if let Some(sort) = sort {\n\tquery.push("sort", Some(sort));\n}\n'
            'impl TryFrom<Entry> for Manga {\n\tfn try_from(e: Entry) -> Result<Self> {\n\t\tlet key = e.key().ok_or_else(|| error!("no key"))?;\n\t}\n}\n'
            'impl From<Entry> for Manga {\n\tfn from(e: Entry) -> Self {\n\t\tlet description = e.text.unwrap_or_default();\n\t}\n}\n'
            'Chapter {\n\tkey,\n\ttitle: name.unwrap_or_default(),\n\t..Default::default()\n};\n'
            'title.as_ref().zip(href).map(|(name, id)| Listing {\n\tid,\n\tname: name.clone(),\n\t..Default::default()\n});\n'
            'let date = parse_date(text, "yyyy-MM-dd");\n'
            'let date = parse_date_with_options(text, "MMM d, yyyy", "en_US_POSIX", "current");\n'
            'let list: List = Request::get(url)?.send()?.json_owned()?;\n'
            'let data: Next = serde_json::from_str(&script.data().unwrap_or_default())?;\n'
            'MultiSelectFilter {\n\tis_genre: true,\n\tuses_tag_style: true,\n\t..Default::default()\n}\n'
            'let rest = url.strip_prefix(BASE_URL)?;\n'
            'Request::get(url)?.send()?;\n'
            'let text = lines.map(|line| line.escape_markdown()).collect::<Vec<_>>().join("\\n\\n");\n'
            'content: PageContent::text(text),\n'
            'web_view.eval_async(&format!("window.state.open({path})"))?;\n'
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

    def test_chrono_dependency_is_noted(self):
        checks.notes.clear()
        path = self.source / 'Cargo.toml'
        path.write_text('[dependencies]\nchrono = "0.4"\n')
        checks.check_cargo(path)
        self.assertTrue(any('parse_date' in n for n in checks.notes))
        checks.notes.clear()
        path.write_text('[dependencies]\nserde = "1"\n')
        checks.check_cargo(path)
        self.assertEqual(checks.notes, [])

    def test_script_file_in_a_source_fails(self):
        (self.source / 'src').mkdir()
        (self.source / 'src/lib.rs').write_text('const SCRIPT: &str = "document.title";\n')
        checks.check_script_files(self.source)
        self.assertEqual(checks.failures, [])
        (self.source / 'src/reader.js').write_text('window.pages = [];\n')
        checks.check_script_files(self.source)
        self.assertTrue(any('reader.js' in failure for failure in checks.failures))

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
        _, notes = self.naming('const PAGE_SIZE: i32 = 24;\n\nfn a() { query.push("limit", Some(&PAGE_SIZE.to_string())); }\n')
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
        _, notes = self.naming('use aidoku::imports::std::current_date;\n\nfn d(e: aidoku::imports::html::Element) {}\n')
        self.assertEqual(notes, [])

    def test_silenced_dead_code_fails(self):
        (self.source / 'src').mkdir()
        for attribute in ('#[allow(dead_code)]', '#![allow(unused_imports)]', '#[expect(unused)]'):
            checks.failures.clear()
            (self.source / 'src/lib.rs').write_text(attribute + '\nfn x() {}\n')
            checks.check_rust(self.source, False)
            self.assertTrue(any('silences' in failure for failure in checks.failures), attribute)




class SurvivingMutantTests(unittest.TestCase):
    """One failing and one passing case for each rule a mutation run showed no test could catch."""

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        checks.ROOT = Path(self.temp.name)
        sources = patch.object(checks, 'SOURCES', checks.ROOT / 'sources')
        sources.start()
        self.addCleanup(sources.stop)
        checks.failures.clear()
        checks.notes.clear()
        self.source = checks.SOURCES / 'en.example'
        (self.source / 'res').mkdir(parents=True)
        (self.source / 'src').mkdir()

    def outcome(self, check, *args):
        checks.failures.clear()
        check(*args)
        return list(checks.failures)

    def write(self, name, text):
        path = self.source / name
        path.write_text(text)
        return path

    def assert_rule(self, check, bad, good, *args, expect=None):
        failures = self.outcome(check, bad, *args)
        self.assertTrue(failures, f'{bad} should fail')
        if expect:
            self.assertTrue(any(expect in failure for failure in failures), failures)
        self.assertEqual(self.outcome(check, good, *args), [], f'{good} should pass')

    def test_text_and_json_format(self):
        self.assert_rule(checks.check_text_file, self.write('a.txt', 'x'), self.write('b.txt', 'x\n'), expect='newline')
        self.assert_rule(checks.check_json, self.write('a.json', '{\n'), self.write('b.json', '{}\n'), expect='not valid JSON')
        self.assert_rule(checks.check_json, self.write('c.json', '{\n  "a": 1\n}\n'), self.write('d.json', '{\n\t"a": 1\n}\n'), expect='spaces')

    def test_source_json_fields(self):
        good = {'id': 'en.example', 'minAppVersion': '0.7.1', 'languages': ['en']}
        cases = [
            ({**good, 'breakingChangeVersion': 2}, 'top-level config'),
            ({key: value for key, value in good.items() if key != 'minAppVersion'}, 'minAppVersion'),
            ({**good, 'id': 'en.other'}, 'info.id'),
            ({**good, 'languages': ['multi']}, 'lists "multi"'),
        ]
        for info, expect in cases:
            with self.subTest(expect=expect):
                path = self.write('res/source.json', json.dumps({'info': info}) + '\n')
                self.assertTrue(any(expect in failure for failure in self.outcome(checks.check_source_json, path, 'en.example')))
        for document, expect in [({'info': good, 'config': {'languageSelectType': 'multi'}}, 'languageSelectType'),
                                 ({'info': good, 'listings': [{'id': 'a'}, {'id': 'a'}]}, 'repeat')]:
            with self.subTest(expect=expect):
                path = self.write('res/source.json', json.dumps(document) + '\n')
                self.assertTrue(any(expect in failure for failure in self.outcome(checks.check_source_json, path, 'en.example')))
        path = self.write('res/source.json', json.dumps({'info': good, 'config': {'languageSelectType': 'single'}}) + '\n')
        self.assertEqual(self.outcome(checks.check_source_json, path, 'en.example'), [])

    def test_filter_defaults_and_header(self):
        sort = {'type': 'sort', 'id': 'sort', 'options': ['A']}
        self.assert_rule(checks.check_filters_json,
                         self.write('a.json', json.dumps([{**sort, 'default': {'index': 0, 'ascending': False}}]) + '\n'),
                         self.write('b.json', json.dumps([{**sort, 'default': {'index': 1, 'ascending': False}}]) + '\n'),
                         expect='framework already uses')
        check = {'type': 'check', 'id': 'completed', 'name': 'Completed'}
        self.assert_rule(checks.check_filters_json, self.write('c.json', json.dumps([check]) + '\n'),
                         self.write('d.json', json.dumps([{**check, 'hideFromHeader': True}]) + '\n'), expect='hideFromHeader')

    def test_rating_select_hides_adult_levels_on_a_source_rated_one(self):
        item = {'type': 'select', 'key': 'contentRating', 'values': ['safe', 'suggestive', 'erotica']}
        self.assert_rule(checks.check_settings_json, self.write('a.json', json.dumps([{**item, 'default': 'erotica'}]) + '\n'),
                         self.write('b.json', json.dumps([{**item, 'default': 'suggestive'}]) + '\n'), 1, expect='contentRating 1')
        self.assertEqual(self.outcome(checks.check_settings_json, self.write('c.json', json.dumps([{**item, 'default': 'erotica'}]) + '\n'), 2), [])

    def test_label_novels_defaults_on(self):
        item = {'type': 'switch', 'key': 'labelNovels', 'title': 'Label Novels'}
        self.assert_rule(checks.check_settings_json, self.write('a.json', json.dumps([{**item, 'default': False}]) + '\n'),
                         self.write('b.json', json.dumps([{**item, 'default': True}]) + '\n'), 1, expect='Label Novels')

    def test_icon_must_be_a_128_png(self):
        def png(width, height):
            return b'\x89PNG\r\n\x1a\n' + b'\0' * 8 + width.to_bytes(4, 'big') + height.to_bytes(4, 'big')
        icon = self.source / 'res/icon.png'
        self.assertTrue(any('is missing' in failure for failure in self.outcome(checks.check_icon, icon)))
        icon.write_bytes(b'GIF89a' + b'\0' * 20)
        self.assertTrue(any('not a PNG' in failure for failure in self.outcome(checks.check_icon, icon)))
        icon.write_bytes(png(256, 256))
        self.assertTrue(any('128x128' in failure for failure in self.outcome(checks.check_icon, icon)))
        icon.write_bytes(png(128, 128))
        self.assertEqual(self.outcome(checks.check_icon, icon), [])

    def test_inline_response_json(self):
        path = self.source / 'src/lib.rs'
        bad = 'let x: T = serde_json::from_str(&Request::get(url)?.string()?)?;\n'
        self.assertTrue(self.outcome(checks.check_api_use, path, bad))
        self.assertEqual(self.outcome(checks.check_api_use, path, 'let x: T = Request::get(url)?.json_owned()?;\n'), [])

    def test_rust_line_rules(self):
        (self.source / 'src/settings.rs').write_text('fn a() {}\n')
        lib = self.source / 'src/lib.rs'
        cases = [
            ('// ----- Helpers -----\n', 'decorative divider'),
            ('let c = HomeComponentValue::MangaList { page_size: Some(10) };\n', 'page_size 10'),
            ('let c = Chapter { language: Some("en".into()) };\n', 'chapter language'),
            ('let a = defaults_get::<String>("a");\n', 'outside settings.rs'),
            ('bail!(AidokuError::Unimplemented);\n', 'Unimplemented'),
        ]
        for line, expect in cases:
            with self.subTest(expect=expect):
                lib.write_text(line)
                self.assertTrue(any(expect in failure for failure in self.outcome(checks.check_rust, self.source, True)))
        lib.write_text('// Explains why.\nlet c = HomeComponentValue::MangaList { page_size: Some(5) };\n')
        self.assertEqual(self.outcome(checks.check_rust, self.source, True), [])
        lib.write_text('let c = Chapter { language: Some("en".into()) };\n')
        self.assertEqual(self.outcome(checks.check_rust, self.source, False), [])

    def test_lock_lists_every_path_dependency(self):
        (self.source / 'Cargo.toml').write_text('[dependencies]\ntheme = { path = "../../templates/theme" }\n')
        template = checks.ROOT / 'templates/theme'
        template.mkdir(parents=True)
        (template / 'Cargo.toml').write_text('[package]\nname = "theme"\nversion = "0.1.0"\n')
        lock = self.source / 'Cargo.lock'
        lock.write_text('version = 4\n')
        self.assertTrue(any("'theme' is missing" in failure for failure in self.outcome(checks.check_template_locks, self.source)))
        lock.write_text('version = 4\n[[package]]\nname = "theme"\nversion = "0.1.0"\n')
        self.assertEqual(self.outcome(checks.check_template_locks, self.source), [])

    def test_readme_row_and_version(self):
        (self.source / 'Cargo.toml').write_text('[package]\nname = "example"\n')
        self.write('res/source.json', json.dumps({'info': {'id': 'en.example', 'version': 3, 'minAppVersion': '0.7.1', 'languages': ['en']}}) + '\n')
        readme = checks.ROOT / 'README.md'
        link = 'sources/en.example-v{0}.aix">.aix (v{0})</a> sources/en.example-v{0}.aix'
        for text, expect in [('', 'has no row'), (link.format(2), 'links v2'),
                             (link.format(3) + ' sources/en.example-v2.aix', 'more than one version')]:
            with self.subTest(expect=expect):
                readme.write_text(text + '\n')
                failures = self.outcome(checks.check_source, 'en.example', None)
                self.assertTrue(any(expect in failure for failure in failures), failures)
        readme.write_text(link.format(3) + '\n')
        failures = self.outcome(checks.check_source, 'en.example', None)
        self.assertFalse([failure for failure in failures if 'README' in failure], failures)
        self.assertTrue(any('has no .cargo/config.toml' in failure for failure in failures), failures)

    def test_single_short_helper_module_note(self):
        self.write(
            'src/helpers.rs',
            'pub fn parse_page_list(body: &str) -> Vec<u8> {\n\tbody.as_bytes().to_vec()\n}\n',
        )
        checks.notes.clear()
        self.assertEqual(self.outcome(checks.check_rust, self.source, True), [])
        self.assertTrue(any('one short helper function' in item for item in checks.notes), checks.notes)
        self.write(
            'src/helpers.rs',
            'pub fn first() -> u8 {\n\t1\n}\n\npub fn second() -> u8 {\n\t2\n}\n',
        )
        checks.notes.clear()
        self.assertEqual(self.outcome(checks.check_rust, self.source, True), [])
        self.assertFalse(any('one short helper function' in item for item in checks.notes), checks.notes)

    def test_empty_string_allocated_before_filter_and_query_concat_notes(self):
        self.write(
            'src/lib.rs',
            'fn bad(name: &str, qs: &str) -> (Option<String>, String) {\n'
            '\t(Some(name.into()).filter(|s: &String| !s.is_empty()), format!("{BASE_URL}/search?{qs}&page=1"))\n'
            '}\n',
        )
        checks.notes.clear()
        self.assertEqual(self.outcome(checks.check_rust, self.source, True), [])
        self.assertTrue(any('allocates a String before checking is_empty()' in item for item in checks.notes), checks.notes)
        self.assertTrue(any('appends & after a query string variable' in item for item in checks.notes), checks.notes)
        self.write(
            'src/lib.rs',
            'fn good(name: &str, qs: &str) -> (Option<String>, String) {\n'
            '\t((!name.is_empty()).then(|| name.into()), format!("{BASE_URL}/search?{qs}"))\n'
            '}\n',
        )
        checks.notes.clear()
        self.assertEqual(self.outcome(checks.check_rust, self.source, True), [])
        self.assertFalse(any('allocates a String before checking is_empty()' in item for item in checks.notes), checks.notes)
        self.assertFalse(any('appends & after a query string variable' in item for item in checks.notes), checks.notes)


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
