"""Create Tauri updater metadata from signed native release artifacts."""
import argparse
from pathlib import Path
from datetime import datetime, timezone
import hashlib
import json
import urllib.parse
import tomllib
import re


def normalize_asset_names(root):
    # GitHub rewrites spaces and special characters during upload. Rename copies
    # before generating URLs and checksums so the feed points at actual assets.
    for file in sorted(root.rglob('*')):
        if not file.is_file():
            continue
        name = re.sub(r'[^A-Za-z0-9._-]', '-', file.name).strip('.')
        if not name:
            raise ValueError(f'Invalid release asset name: {file.name}')
        target = file.with_name(name)
        if target != file:
            if target.exists():
                raise ValueError(f'Normalized release asset name collision: {name}')
            file.rename(target)


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
    repository_root = Path(__file__).resolve().parents[1]
    versions = [tomllib.loads((repository_root/'Cargo.toml').read_text(encoding='utf-8'))['package']['version'],
                json.loads((repository_root/'tauri.conf.json').read_text(encoding='utf-8'))['version'],
                json.loads((repository_root/'ui/package.json').read_text(encoding='utf-8'))['version']]
    if any(version != args.version for version in versions):
        raise ValueError(f'Release tag and application versions disagree: {args.version}, {versions}')
    output = args.artifacts
    normalize_asset_names(output)
    changelog = (repository_root/'CHANGELOG.md').read_text(encoding='utf-8')
    heading = f'## [{args.version}]'
    if heading not in changelog:
        raise ValueError('A versioned changelog entry is required before publishing')
    notes = changelog.split(heading, 1)[1].split('\n## [', 1)[0].partition('\n')[2].strip()
    feed = manifest(output, args.version, args.repository)
    feed['notes'] = notes
    (output/'latest.json').write_text(json.dumps(feed, indent=2)+'\n', encoding='utf-8')
    files = sorted(path for path in output.rglob('*') if path.is_file() and path.name != 'SHA256SUMS')
    names = [p.name for p in files]
    if len(names) != len(set(names)): raise ValueError('Release asset names must be unique')
    (output/'SHA256SUMS').write_text(''.join(hashlib.sha256(p.read_bytes()).hexdigest()+'  '+p.name+'\n' for p in files), encoding='utf-8')
