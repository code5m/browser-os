"""Synthetic source parsing, secret separation, endpoint normalization and scanner rejection."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import subprocess
import contextlib
import io
from unittest.mock import patch

def load(name, file):
    spec=importlib.util.spec_from_file_location(name,Path(__file__).parents[1]/'scripts'/file)
    module=importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module

importer=load('ai_import','import-local-ai.py')
scanner=load('ai_scan','check-local-ai-secrets.py')

class ImportTests(unittest.TestCase):
    def test_import_separates_credentials_and_deduplicates(self):
        with tempfile.TemporaryDirectory() as temp:
            home=Path(temp)
            source=home/'.codebuddy'
            source.mkdir()
            key='synthetic-ai-credential-for-import-tests'
            row=dict(id='fixture-chat',name='Fixture',url='https://provider.invalid/v1/chat/completions',apiKey=key)
            (source/'models.json').write_text(json.dumps(dict(models=[row,row])))
            plugin=home/'.workbuddy/skills/fixture'
            plugin.mkdir(parents=True)
            (plugin/'credentials.env').write_text('export API_TOKEN="'+key+'"\n')
            data, credentials=importer.discover(home)
            self.assertEqual(len(data['profiles']),1)
            self.assertEqual(data['profiles'][0]['base_url'],'https://provider.invalid/v1')
            self.assertEqual(len(credentials),2)
            self.assertNotIn(key,json.dumps(data))
            self.assertEqual(scanner.findings(key.encode(),{key.encode()}),'KNOWN_LOCAL_AI_CREDENTIAL')
            self.assertIsNone(scanner.findings(json.dumps(data).encode(),{key.encode()}))

    def test_rejects_dynamic_values_and_credential_urls(self):
        for value in ['${API_KEY}','your-api-key-placeholder','os.environ.get("API_KEY")']:
            self.assertFalse(importer.usable(value))
        for url in ['https://user:pass@provider.invalid/v1','http://provider.invalid','https://provider.invalid?key=abc']:
            with self.assertRaises(ValueError):importer.endpoint(url)
        self.assertEqual(scanner.findings(b'sk-'+b'A'*36,set()),'PROVIDER_TOKEN_LITERAL')
        self.assertIsNone(scanner.findings(b'const apiKey = userInput;',set()))

    def test_staged_guard_blocks_without_printing_value(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp)
            subprocess.run(['git','init','-q',str(root)],check=True)
            secret=b'synthetic-staged-credential-for-test'
            (root/'accidental.txt').write_bytes(secret)
            subprocess.run(['git','-C',str(root),'add','accidental.txt'],check=True)
            previous_root, previous_lookup=scanner.ROOT,scanner.known_credentials
            scanner.ROOT=root
            scanner.known_credentials=lambda:{secret}
            output=io.StringIO()
            try:
                with contextlib.redirect_stdout(output):self.assertTrue(scanner.check(staged=True))
            finally:
                scanner.ROOT,scanner.known_credentials=previous_root,previous_lookup
            self.assertIn('KNOWN_LOCAL_AI_CREDENTIAL',output.getvalue())
            self.assertNotIn(secret.decode(),output.getvalue())


class SecretScannerTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix='browseros-secret-guard-')
        self.addCleanup(self.directory.cleanup)
        self.home = Path(self.directory.name)
        self.metadata = self.home / 'ai-profiles.json'
        mocked_path = patch.object(scanner, 'metadata_path', return_value=self.metadata)
        mocked_path.start()
        self.addCleanup(mocked_path.stop)

    def write_metadata(self, profiles=None, plugins=None):
        self.metadata.write_text(json.dumps({
            'profiles': profiles or [], 'bindings': {}, 'plugins': plugins or [],
        }), encoding='utf-8')

    def test_absent_config_needs_no_keyring_but_scans_discovered_bearer(self):
        source = self.home / '.codebuddy'
        source.mkdir()
        secret = 'synthetic-discovered-credential-123456'
        (source / 'models.json').write_text(json.dumps({'models': [{
            'id': 'fixture', 'name': 'Fixture', 'url': 'https://provider.invalid/v1',
            'apiKey': 'Bearer ' + secret,
        }]}))
        def unexpected_lookup(_reference):
            self.fail('absent metadata must not open a credential backend')
        values = scanner.known_credentials(self.home, unexpected_lookup)
        self.assertIn(secret.encode(), values)
        self.assertIn(('Bearer ' + secret).encode(), values)
        self.assertEqual(scanner.findings(secret.encode(), values), 'KNOWN_LOCAL_AI_CREDENTIAL')

    def test_uses_persisted_model_and_plugin_versions_and_legacy_fallback(self):
        model_id, legacy_id, plugin_id = 'a' * 24, 'b' * 24, 'c' * 24
        version = '12345678-1234-4234-8234-123456789abc'
        model_ref, legacy_ref = 'ai:' + model_id + ':' + version, 'ai:' + legacy_id
        plugin_ref = 'ai-plugin:' + plugin_id + ':' + version
        self.write_metadata([
            {'id': model_id, 'credential_ref': model_ref},
            {'id': legacy_id},
        ], [{'credential_refs': [plugin_ref]}])
        values = {
            model_ref: ['Bearer synthetic-model-credential-123456'],
            legacy_ref: ['synthetic-legacy-credential-123456'],
            plugin_ref: ['synthetic-plugin-credential-123456'],
        }
        seen = []
        def lookup(reference):
            seen.append(reference)
            return values[reference]
        secrets = scanner.known_credentials(self.home, lookup)
        self.assertEqual(set(seen), set(values))
        self.assertNotIn('ai:' + model_id, seen, 'a versioned credential must not read an obsolete alias')
        for secret in values.values():
            self.assertIn(secret[0].encode(), secrets)
        self.assertIn(b'synthetic-model-credential-123456', secrets)
        self.assertEqual(scanner.findings(b'synthetic-plugin-credential-123456', secrets), 'KNOWN_LOCAL_AI_CREDENTIAL')

    def test_invalid_and_cross_owner_references_fail_before_keyring_reads(self):
        owner = 'a' * 24
        malformed = [
            {'profiles': [{'id': owner, 'credential_ref': 'ai:' + 'b' * 24}]},
            {'profiles': [{'id': owner, 'credential_ref': 'ai:' + owner + ':not-a-uuid'}]},
            {'profiles': [{'id': owner, 'credential_ref': None}]},
            {'plugins': [{'credential_refs': ['db:' + 'c' * 24]}]},
            {'plugins': [{'credential_refs': 'ai-plugin:' + 'c' * 24}]},
        ]
        for payload in malformed:
            with self.subTest(payload=payload):
                self.metadata.write_text(json.dumps({'profiles': [], 'bindings': {}, **payload}))
                with self.assertRaises(ValueError):
                    scanner.known_credentials(self.home, lambda _: self.fail('invalid metadata reached the keyring'))

    def test_reference_versions_and_legacy_plugins_match_native_parser(self):
        owner, plugin = 'A' * 24, 'b' * 24
        version = '12345678-1234-4234-8234-123456789aBc'
        for form in [version, version.replace('-', ''), '{' + version + '}', 'urn:uuid:' + version]:
            model_ref = 'ai:' + owner + ':' + form
            with self.subTest(form=form):
                refs = scanner.credential_references({
                    'profiles': [{'id': owner, 'credential_ref': model_ref}], 'bindings': {},
                    'plugins': [{'credential_refs': ['ai-plugin:' + plugin]}],
                })
                self.assertEqual(refs, {model_ref, 'ai-plugin:' + plugin})
        for form in [version, version.replace('-', ''), '{' + version + '}']:
            plugin_ref = 'ai-plugin:' + plugin + ':' + form
            self.assertEqual(scanner.credential_references({
                'profiles': [], 'bindings': {}, 'plugins': [{'credential_refs': [plugin_ref]}],
            }), {plugin_ref})
        for invalid in ['-' + version, '{' + version.replace('-', '') + '}', 'urn:uuid:urn:uuid:' + version]:
            with self.assertRaises(ValueError):
                scanner.credential_references({
                    'profiles': [{'id': owner, 'credential_ref': 'ai:' + owner + ':' + invalid}], 'bindings': {},
                })
        with self.assertRaises(ValueError):
            scanner.credential_references({
                'profiles': [], 'bindings': {},
                'plugins': [{'credential_refs': ['ai-plugin:' + plugin + ':urn:uuid:' + version]}],
            })

    def test_existing_plugin_catalog_is_not_subject_to_import_batch_limits(self):
        refs = ['ai-plugin:' + format(index, '024x') for index in range(65)]
        config = {
            'profiles': [], 'bindings': {},
            'plugins': [{'credential_refs': refs} for _ in range(257)],
        }
        self.assertEqual(scanner.credential_references(config), set(refs))

    def test_unreadable_keyring_and_malformed_metadata_fail_closed(self):
        self.write_metadata([{'id': 'a' * 24}])
        with self.assertRaises(OSError):
            scanner.known_credentials(self.home, lambda _: (_ for _ in ()).throw(OSError('unavailable')))
        self.metadata.write_text('{invalid json')
        with self.assertRaises(ValueError):
            scanner.known_credentials(self.home, lambda _: [])

    def test_non_file_and_oversized_metadata_fail_closed(self):
        self.metadata.mkdir()
        with self.assertRaises(ValueError):
            scanner.known_credentials(self.home, lambda _: self.fail('unsafe metadata reached keyring'))
        self.metadata.rmdir()
        self.metadata.write_bytes(b' ' * (scanner.MAX_METADATA_BYTES + 1))
        with self.assertRaises(ValueError):
            scanner.known_credentials(self.home, lambda _: self.fail('oversized metadata reached keyring'))

    def test_empty_config_does_not_require_a_native_backend(self):
        self.write_metadata()
        self.assertEqual(scanner.known_credentials(self.home, lambda _: self.fail('no references')), set())

    def test_metadata_location_matches_native_app_data_contract(self):
        # Exercise the path resolver itself independently from the fixture patch.
        resolve = self._metadata_resolver
        home = self.home
        self.assertEqual(resolve(home, 'linux', {}), home / '.local/share/com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json')
        self.assertEqual(resolve(home, 'linux', {'XDG_DATA_HOME': str(home / 'fixture-data')}),
                         home / 'fixture-data/com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json')
        self.assertEqual(resolve(home, 'linux', {'XDG_DATA_HOME': 'relative'}),
                         home / '.local/share/com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json')
        self.assertEqual(resolve(home, 'win32', {'APPDATA': str(home / 'fixture-roaming')}),
                         home / 'fixture-roaming/com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json')
        self.assertEqual(resolve(home, 'darwin', {}),
                         home / 'Library/Application Support/com.jizhijiandan.mvp/mvp-browser-os/ai-profiles.json')

    _metadata_resolver = staticmethod(scanner.metadata_path)

if __name__=='__main__':unittest.main()
