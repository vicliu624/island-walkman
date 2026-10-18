"""Launch a real desktop window and verify WebView, React, IPC and clean close."""
import argparse
import json
from pathlib import Path
import subprocess
import tempfile

parser = argparse.ArgumentParser()
parser.add_argument('--executable', required=True, type=Path)
parser.add_argument('--verify-release', action='store_true')
args = parser.parse_args()
with tempfile.TemporaryDirectory(prefix='island-desktop-smoke-') as folder:
    report = Path(folder)/'desktop-result.json'
    command = [str(args.executable.resolve()), '--desktop-smoke', '--smoke-report', str(report), '--data', folder]
    if args.verify_release:
        command.append('--verify-release')
    try:
        subprocess.run(command, check=True, timeout=120 if args.verify_release else 60)
    except subprocess.TimeoutExpired:
        print('Desktop result before timeout:', report.read_text(encoding='utf-8') if report.exists() else 'No IPC report received', flush=True)
        raise
    assert report.exists(), 'Desktop exited without validating its window (possibly an existing instance)'
    result = json.loads(report.read_text(encoding='utf-8'))
    assert result['passed'], result.get('error')
print('Desktop smoke passed: native window, React, IPC, write gate and close-to-exit.')
