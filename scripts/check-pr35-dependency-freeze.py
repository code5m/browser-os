#!/usr/bin/env python3
"""Verify the exact reviewed build-only dependency and rejection boundaries."""
from __future__ import annotations
import copy
import importlib.util
import json
from pathlib import Path
import sys
import unittest

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / 'scripts'))
from dependency_lock_policy import lock_declared_deps_match

class FrozenBuildDependencyTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.package = json.loads((ROOT / 'package.json').read_text())
        cls.lock_text = (ROOT / 'package-lock.json').read_text()
        cls.families = []
        for name in ('image-preview', 'script-domain', 'command-domain'):
            spec = importlib.util.spec_from_file_location(
                'freeze_' + name.replace('-', '_'), ROOT / f'scripts/check-{name}-policy.py')
            module = importlib.util.module_from_spec(spec)
            spec.loader.exec_module(module)
            cls.families.append((name, module))

    def test_pinned_build_only_dependency(self):
        self.assertEqual(self.package['devDependencies']['lightningcss'], '1.33.0')
        self.assertNotIn('lightningcss', self.package['dependencies'])
        lock = json.loads(self.lock_text)
        self.assertEqual(lock['packages']['node_modules/lightningcss']['version'], '1.33.0')
        self.assertTrue(lock['packages']['node_modules/lightningcss']['dev'])
        self.assertTrue(lock['packages']['node_modules/lightningcss']['integrity'].startswith('sha512-'))
        for name, module in self.families:
            with self.subTest(family=name):
                self.assertEqual(module.extract_deps(json.dumps(self.package)), module.BASELINE_DEPS)
                self.assertTrue(lock_declared_deps_match(self.lock_text, module.BASELINE_DEPS))

    def test_unreviewed_add_remove_version_or_scope_is_rejected(self):
        mutations = []
        changed = copy.deepcopy(self.package)
        changed['devDependencies']['lightningcss'] = '^1.33.0'
        mutations.append(('version-range', changed))
        changed = copy.deepcopy(self.package)
        changed['devDependencies'].pop('lightningcss')
        mutations.append(('removal', changed))
        changed = copy.deepcopy(self.package)
        changed['dependencies']['lightningcss'] = changed['devDependencies'].pop('lightningcss')
        mutations.append(('runtime-move', changed))
        changed = copy.deepcopy(self.package)
        changed['devDependencies']['unreviewed-tool'] = '1.0.0'
        mutations.append(('new-dependency', changed))
        for name, module in self.families:
            for mutation, package in mutations:
                with self.subTest(family=name, mutation=mutation):
                    self.assertNotEqual(module.extract_deps(json.dumps(package)), module.BASELINE_DEPS)
                    lock = json.loads(self.lock_text)
                    for field in ('dependencies', 'devDependencies', 'peerDependencies', 'optionalDependencies'):
                        lock['packages'][''][field] = package.get(field, {})
                    self.assertFalse(lock_declared_deps_match(json.dumps(lock), module.BASELINE_DEPS))

    def test_css_optimizer_is_not_imported_by_runtime_source(self):
        for directory in ('src', 'packages'):
            for path in (ROOT / directory).rglob('*'):
                if path.is_file() and path.suffix in ('.ts', '.js', '.mjs', '.vue'):
                    self.assertNotIn('lightningcss', path.read_text(), str(path.relative_to(ROOT)))

if __name__ == '__main__':
    unittest.main(verbosity=2)
