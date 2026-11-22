"""Behavior regression definitions for the read-only native import discoverer.

They use synthetic credentials and temporary source files only.
"""
import contextlib
import importlib.util
import io
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch

SOURCE = Path(__file__).resolve().parents[1] / "scripts" / "import-local-ai.py"
SPEC = importlib.util.spec_from_file_location("browseros_import_local_ai", SOURCE)
IMPORTER = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(IMPORTER)

TOKEN = "synthetic-native-credential-1234567890"
REQUEST_ID = "a" * 32


class NativeImportDiscoveryTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="browseros-import-")
        self.addCleanup(self.directory.cleanup)
        self.home = Path(self.directory.name)
        self.source = self.home / ".codebuddy" / "models.json"
        self.source.parent.mkdir()

    def model(self, name="Local fixture", secret=TOKEN):
        payload = {"models": [{
            "id": "fixture-model",
            "name": name,
            "url": "https://provider.invalid/v1",
            "apiKey": secret,
        }]}
        self.source.write_text(json.dumps(payload), encoding="utf-8")

    def request(self, **extra):
        request = {"protocol": IMPORTER.PROTOCOL, "request_id": REQUEST_ID}
        request.update(extra)
        return json.dumps(request).encode("utf-8")

    def test_model_metadata_rejects_complete_secret(self):
        self.model(name=TOKEN)
        with self.assertRaisesRegex(RuntimeError, "^secret present in metadata$"):
            IMPORTER.discover(self.home)

    def test_model_metadata_rejects_bare_token_from_bearer_value(self):
        self.model(name=TOKEN, secret="Bearer " + TOKEN)
        with self.assertRaisesRegex(RuntimeError, "^secret present in metadata$"):
            IMPORTER.discover(self.home)

    def test_plugin_metadata_rejects_bare_token_from_authorization(self):
        self.model(secret="synthetic-plugin-credential-0987654321")
        plugin = self.home / ".codebuddy" / "mcp.json"
        plugin.write_text(json.dumps({"mcpServers": {
            TOKEN: {"type": "stdio", "env": {"Authorization": "Bearer " + TOKEN}}
        }}), encoding="utf-8")
        with self.assertRaisesRegex(RuntimeError, "^secret present in metadata$"):
            IMPORTER.discover(self.home)

    def test_native_discovery_preserves_values_for_the_native_owner(self):
        self.model(secret="Bearer " + TOKEN)
        plugin = self.home / ".codebuddy" / "mcp.json"
        plugin.write_text(json.dumps({"mcpServers": {
            "local-fixture": {"type": "stdio", "env": {"Authorization": "Bearer " + TOKEN}}
        }}), encoding="utf-8")
        original_model, original_plugin = self.source.read_bytes(), plugin.read_bytes()
        payload = json.loads(IMPORTER.native_export(self.home, self.request()))
        self.assertEqual(payload["protocol"], IMPORTER.PROTOCOL)
        self.assertEqual(payload["request_id"], REQUEST_ID)
        self.assertEqual(len(payload["profiles"]), 1)
        self.assertEqual(len(payload["plugins"]), 1)
        self.assertEqual(payload["credentials"][payload["profiles"][0]["credential_ref"]], "Bearer " + TOKEN)
        self.assertEqual(payload["credentials"][payload["plugins"][0]["credential_refs"][0]], "Bearer " + TOKEN)
        public = json.dumps({key: value for key, value in payload.items() if key != "credentials"})
        self.assertNotIn(TOKEN, public)
        self.assertNotIn(TOKEN, json.dumps(IMPORTER.summary(self.home)))
        self.assertEqual(self.source.read_bytes(), original_model)
        self.assertEqual(plugin.read_bytes(), original_plugin)

    def test_native_protocol_rejects_unknown_fields_without_export(self):
        self.model()
        with self.assertRaisesRegex(ValueError, "^unsupported native request$"):
            IMPORTER.native_export(self.home, self.request(unexpected="value"))

    def test_native_output_limit_does_not_return_a_partial_payload(self):
        self.model()
        with patch.object(IMPORTER, "MAX_OUTPUT_BYTES", 32):
            with self.assertRaisesRegex(IMPORTER.DiscoveryLimit, "^output byte limit$"):
                IMPORTER.native_export(self.home, self.request())

    def test_retired_apply_cannot_write_config_or_expose_a_secret(self):
        self.model()
        destination = self.home / "ai-profiles.json"
        original = b'{"profiles":[],"bindings":{}}'
        destination.write_bytes(original)
        output = io.StringIO()
        with patch.object(IMPORTER.sys, "argv", ["import-local-ai.py", "--apply", "--destination", str(destination)]):
            with patch.object(IMPORTER, "discover", side_effect=AssertionError("write entry must stop before discovery")):
                with contextlib.redirect_stdout(output):
                    status = IMPORTER.main()
        self.assertEqual(status, 2)
        self.assertEqual(json.loads(output.getvalue())["error"], "AI_IMPORT_NATIVE_REQUIRED")
        self.assertNotIn(TOKEN, output.getvalue())
        self.assertEqual(destination.read_bytes(), original)


if __name__ == "__main__":
    unittest.main()
