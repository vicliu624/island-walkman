"""Collect dependency notices before Tauri bundles platform resources."""
from pathlib import Path
import json
import shutil
import subprocess
root = Path(__file__).resolve().parents[1]
dest = root/'data/bundle-notices'
dest.mkdir(parents=True, exist_ok=True)
shutil.copytree(root/'licenses', dest/'licenses', dirs_exist_ok=True)
metadata = json.loads(subprocess.check_output(['cargo', 'metadata', '--format-version', '1', '--locked'], cwd=root))
rows = []
for package in metadata['packages']:
    rows.append({key: package.get(key) for key in ['name', 'version', 'license', 'repository']})
    source = Path(package['manifest_path']).parent
    if source == root: continue
    for pattern in ['LICENSE*', 'COPYING*', 'NOTICE*']:
        for file in source.glob(pattern):
            if file.is_file():
                target = dest/'rust'/(package['name']+'-'+package['version'])/file.name
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy2(file, target)
(dest/'rust-packages.json').write_text(json.dumps(rows, indent=2), encoding='utf-8')
for file in (root/'ui/node_modules').rglob('*'):
    if file.is_file() and file.name.upper().startswith(('LICENSE', 'COPYING', 'NOTICE')):
        target = dest/'npm'/file.relative_to(root/'ui/node_modules')
        target.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(file, target)
print('Collected bundled license texts.')
