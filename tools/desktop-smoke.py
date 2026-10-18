"""Launch a real desktop window and verify WebView, React, IPC and clean close."""
import argparse
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--executable', required=True, type=Path)
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='island-desktop-smoke-') as folder:
    subprocess.run([str(args.executable.resolve()), '--desktop-smoke', '--data', folder], check=True, timeout=60)
print('Desktop smoke passed: native window, React, IPC, write gate and close-to-exit.')
