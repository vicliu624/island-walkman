"""Create Tauri updater metadata from signed native release artifacts."""
import argparse
from pathlib import Path
from datetime import datetime, timezone
import hashlib
import json
import urllib.parse


def manifest(root, version, repository):
    platforms = {}
    for signature in root.rglob('*.sig'):
        artifact = signature.with_suffix('')
        if not artifact.is_file(): raise ValueError(f'Missing signed artifact: {artifact}')
        name = artifact.name
        if name.endswith('.exe'): key = 'windows-x86_64'
        elif name.endswith('.AppImage'): key = 'linux-x86_64-appimage'
        elif name.endswith('.deb'): key = 'linux-x86_64-deb'
        elif name.endswith('.app.tar.gz'):
            key = 'darwin-aarch64' if 'aarch64' in str(artifact) else 'darwin-x86_64'
        else: continue
        if key in platforms: raise ValueError(f'Duplicate updater target: {key}')
        platforms[key] = {'signature': signature.read_text().strip(), 'url': f'https://github.com/{repository}/releases/download/v{version}/'+urllib.parse.quote(name)}
    expected = {'windows-x86_64', 'linux-x86_64-appimage', 'linux-x86_64-deb', 'darwin-aarch64', 'darwin-x86_64'}
    if platforms.keys() != expected: raise ValueError(f'Incomplete release: {platforms.keys()}')
    return {'version': version, 'notes': 'Native desktop application, optional Windows shortcuts and signed GitHub Release updates. See CHANGELOG.md.', 'pub_date': datetime.now(timezone.utc).isoformat(), 'platforms': platforms}

if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--artifacts', required=True, type=Path)
    parser.add_argument('--version', required=True)
    parser.add_argument('--repository', default='vicliu624/island-walkman')
    args = parser.parse_args()
    output = args.artifacts
    (output/'latest.json').write_text(json.dumps(manifest(output, args.version, args.repository), indent=2)+'\n', encoding='utf-8')
    files = sorted(path for path in output.rglob('*') if path.is_file() and path.name != 'SHA256SUMS')
    names = [p.name for p in files]
    if len(names) != len(set(names)): raise ValueError('Release asset names must be unique')
    (output/'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.name+'\n' for p in files), encoding='utf-8')
