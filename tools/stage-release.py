"""Stage only installable desktop bundles and signed updater artifacts."""
import argparse
from pathlib import Path
import shutil
import json
import re
p = argparse.ArgumentParser()
p.add_argument('--bundle', required=True, type=Path)
p.add_argument('--arch', required=True)
p.add_argument('--output', default='dist/native', type=Path)
p.add_argument('--version')
a = p.parse_args()
version=a.version or json.loads((Path(__file__).resolve().parents[1]/'tauri.conf.json').read_text(encoding='utf-8'))['version']
a.output.mkdir(parents=True, exist_ok=True)
for file in a.bundle.rglob('*'):
    if not file.is_file(): continue
    name = file.name
    match=re.search(r'_(\d+\.\d+\.\d+(?:-[A-Za-z0-9.-]+)?)_',name)
    if match and match.group(1)!=version: continue
    if not name.endswith(('.exe', '.exe.sig', '.deb', '.deb.sig', '.AppImage', '.AppImage.sig', '.dmg', '.app.tar.gz', '.app.tar.gz.sig')): continue
    if '.app.tar.gz' in name: name = name.replace('.app.tar.gz', '_'+a.arch+'.app.tar.gz')
    target = a.output/name
    if target.exists(): raise ValueError(f'Refusing to overwrite {target}')
    shutil.copy2(file, target)
print('Staged desktop release assets.')
