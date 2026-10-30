"""Synthetic source parsing, secret separation, endpoint normalization and scanner rejection."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest
import subprocess
import contextlib
import io

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

if __name__=='__main__':unittest.main()
