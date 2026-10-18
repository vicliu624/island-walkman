"""Prevent incomplete or ambiguous updater feeds from reaching Releases."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

spec = importlib.util.spec_from_file_location('release_manifest', Path(__file__).with_name('release-manifest.py'))
module = importlib.util.module_from_spec(spec)
spec.loader.exec_module(module)


class ReleaseManifestTests(unittest.TestCase):
    def seed(self, root):
        for name in ['Island Walkman_0.2.0_x64-setup.exe', 'Island Walkman_0.2.0_amd64.AppImage', 'Island Walkman_0.2.0_amd64.deb',
                     'Island Walkman_aarch64.app.tar.gz', 'Island Walkman_x86_64.app.tar.gz']:
            (root/name).write_bytes(b'fixture')
            (root/(name+'.sig')).write_text('signature-fixture')

    def test_complete_feed_maps_unique_platform_assets(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.seed(root)
            feed = module.manifest(root, '0.2.0', 'owner/repo')
            self.assertEqual(len(feed['platforms']), 5)
            self.assertIn('%20', feed['platforms']['windows-x86_64']['url'])
            self.assertIn('aarch64', feed['platforms']['darwin-aarch64']['url'])

    def test_missing_architecture_rejects_partial_release(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.seed(root)
            (root/'Island Walkman_aarch64.app.tar.gz.sig').unlink()
            with self.assertRaisesRegex(ValueError, 'Incomplete release'):
                module.manifest(root, '0.2.0', 'owner/repo')

    def test_duplicate_platform_rejected(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            self.seed(root)
            (root/'other.exe').write_bytes(b'fixture')
            (root/'other.exe.sig').write_text('signature-fixture')
            with self.assertRaisesRegex(ValueError, 'Duplicate updater target'):
                module.manifest(root, '0.2.0', 'owner/repo')
