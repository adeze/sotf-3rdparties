import json
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

SCRIPTS = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS))
import gpui_provenance
from gpui_toml import InlineTable, dumps_toml


class GpuiToolingTest(unittest.TestCase):
    def test_emitter_preserves_quoted_keys(self):
        document = {
            "features": {"my.feature": ["a"], "contains'quote": [], "unicode-é": []},
            "metadata": {"table.name": InlineTable({"key.with.dot": "value", "space key": True})},
        }
        self.assertEqual(tomllib.loads(dumps_toml(document)), document)

    def test_restoration_in_isolated_tree(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            pre = root / 'pre/gpui/crates/gpui'
            regen = root / 'regen/gpui'
            dest = root / 'dest/gpui'
            for path in (pre, regen, dest):
                path.mkdir(parents=True)
            (pre / 'Cargo.toml').write_text('''[package]
name = "sotf-local"
version = "1.0.0"
[lib]
name = "gpui"
[dependencies]
font-kit = { package = "sotf-zed-font-kit", version = "0.14.1-zed", path = "../../../zed-font-kit" }
''')
            (regen / 'Cargo.toml').write_text('''[package]
name = "gpui"
version = "9.9.9"
publish = false
[dependencies]
font-kit = { git = "https://example.com/font" }
gpui_util = { git = "https://example.com/zed", features = ["x"] }
scap = { git = "https://example.com/scap", optional = true }
[target.'cfg(target_os = "macos")'.dependencies]
serde = "1"
[[bin]]
name = "tool"
path = "src/main.rs"
''')
            command = [sys.executable, str(SCRIPTS / 'restore_sotf_manifests.py'),
                       '--pre', str(root / 'pre'), '--regen', str(root / 'regen'),
                       '--crates', str(root / 'dest')]
            subprocess.run(command, cwd=root, check=True, capture_output=True, text=True)
            result = (dest / 'Cargo.toml').read_text()
            data = tomllib.loads(result)
            self.assertEqual(data['package'], {'name': 'sotf-local', 'version': '1.0.0'})
            self.assertEqual(data['dependencies']['font-kit']['path'], '../../../zed-font-kit')
            self.assertEqual(data['dependencies']['gpui_util'], {'workspace': True, 'features': ['x']})
            self.assertEqual(data['dependencies']['scap']['package'], 'sotf-zed-scap')
            self.assertTrue(data['dependencies']['scap']['optional'])
            self.assertEqual(data['target']['cfg(target_os = "macos")']['dependencies']['serde'], '1')
            self.assertEqual(data['bin'][0]['path'], 'src/main.rs')
            subprocess.run(command, cwd=root, check=True, capture_output=True, text=True)
            self.assertEqual(result, (dest / 'Cargo.toml').read_text())

    def test_provenance_detects_current_field_drift(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / 'gpui/crates/gpui_windows').mkdir(parents=True)
            (root / 'FORKS.json').write_text(json.dumps({'forks': [{
                'directory': 'gpui', 'upstream_revision': 'v1',
                'support_upstream_revision': 'abc', 'members': ['gpui_windows'],
                'support_crates': []}]}))
            (root / 'gpui/crates/gpui_windows/Cargo.toml').write_text('[package]\nname = "sotf-win"\nversion = "1"\n')
            for path in (root / 'gpui/README.md', root / 'gpui/crates/gpui_windows/VENDORING.md'):
                path.write_text('History v0\n' + gpui_provenance.START + '\n' + gpui_provenance.END + '\n')
            self.assertEqual(gpui_provenance.update(root, write=True), [])
            self.assertEqual(gpui_provenance.update(root), [])
            manifest = root / 'gpui/crates/gpui_windows/Cargo.toml'
            manifest.write_text(manifest.read_text().replace('version = "1"', 'version = "2"'))
            self.assertEqual(len(gpui_provenance.update(root)), 2)
            self.assertIn('History v0', (root / 'gpui/README.md').read_text())


if __name__ == '__main__':
    unittest.main()
