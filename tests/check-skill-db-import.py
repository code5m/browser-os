"""Synthetic local fixtures only. No real credential or network access."""
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('skill_import', Path(__file__).parents[1] / 'scripts/import-skill-database-profiles.py')
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class SkillImportTest(unittest.TestCase):
    def test_literal_sources_and_secret_separation(self):
        with tempfile.TemporaryDirectory() as root:
            backend = Path(root) / 'backend'
            yili = Path(root) / 'yili'
            for base in [backend, yili]:
                (base / 'references').mkdir(parents=True)
                (base / 'scripts').mkdir()
            (backend / 'references/connections.md').write_text('''### 1.1 MySQL
| dev | `db.example.com:3306` | `reader` | `fixture-only-password` | test |
| dev duplicate | `db.example.com:3306` | `reader` | `fixture-only-password` | test |
## 3. PostgreSQL
| prod | `pg.example.com:5432` | `app` | `reader` | `fixture-only-password` |
''')
            (yili / 'references/connections.md').write_text('''## 1. MySQL
| prod | `yili.example.com:3306` | `reader` / `fixture-only-password` | blocked |
## 7. PostgreSQL
| uat | `pg.example.com:5432/uat` | `reader` / `fixture-only-password` |
''')
            (yili / 'scripts/transit.sh').write_text('\n'.join(f'{prefix}_HOST="transit.example.com"; {prefix}_PORT={port}\n{prefix}_USER="reader"; {prefix}_PWD="fixture-only-password"' for prefix, port in [('UAT', 9029), ('PROD', 9030), ('KAIMEN', 9031)]))
            rows = module.collect(backend, yili)
            self.assertEqual(len(rows), 7)
            self.assertEqual(len({r['config']['id'] for r in rows}), 7)
            self.assertNotIn('fixture-only-password', json.dumps([r['config'] for r in rows]))
            self.assertTrue(all(not r['config']['allow_write'] and not r['config']['enabled'] for r in rows))
            self.assertTrue(next(r for r in rows if r['config']['host'] == 'yili.example.com')['config']['production_hint'])
            self.assertEqual([r['config']['id'] for r in rows], [r['config']['id'] for r in module.collect(backend, yili)])


unittest.main()
