"""Cached older bundles must never masquerade as current build artifacts."""
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

class StageReleaseTests(unittest.TestCase):
    def test_only_current_version_and_current_architecture_are_staged(self):
        with tempfile.TemporaryDirectory() as folder:
            root=Path(folder);bundle=root/'bundle';bundle.mkdir();output=root/'out'
            names=['Island Walkman_0.2.2_x64-setup.exe','Island Walkman_0.2.2_x64-setup.exe.sig',
                   'Island Walkman_0.3.0_x64-setup.exe','Island Walkman_0.3.0_x64-setup.exe.sig',
                   'Island Walkman.app.tar.gz','Island Walkman.app.tar.gz.sig','unrelated.txt']
            for name in names:(bundle/name).write_bytes(b'synthetic fixture')
            subprocess.run([sys.executable,str(Path(__file__).with_name('stage-release.py')),'--bundle',str(bundle),'--arch','x86_64','--output',str(output),'--version','0.3.0'],check=True,capture_output=True)
            self.assertEqual({p.name for p in output.iterdir()},set(names[2:4])|{'Island Walkman_x86_64.app.tar.gz','Island Walkman_x86_64.app.tar.gz.sig'})
