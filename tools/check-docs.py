"""Keep maintained repository documentation in English and verify required files."""
from pathlib import Path
import re
import subprocess
import tomllib
import importlib.util

root = Path(__file__).resolve().parents[1]
for name in ['README.md','CHANGELOG.md','LICENSE','THIRD_PARTY_NOTICES.md','docs/PROTOCOL.md']:
    assert (root/name).is_file(), f'Missing required document: {name}'
for name in subprocess.check_output(['git','ls-files','--','*.md'],cwd=root,text=True).splitlines():
    text=(root/name).read_text(encoding='utf-8-sig')
    assert not re.search('[\u3040-\u30ff\u3400-\u9fff]',text), f'Non-English maintained documentation: {name}'
    for target in re.findall(r'\]\(([^)]+)\)',text):
        if '://' in target or target.startswith('#'):
            continue
        assert (root/name).parent.joinpath(target.split('#')[0]).exists(), f'Broken relative link in {name}: {target}'
assert '[Unreleased]' in (root/'CHANGELOG.md').read_text()
assert tomllib.loads((root/'Cargo.toml').read_text())['package']['license']=='MIT'
assert (root/'LICENSE').read_text(encoding='utf-8-sig').startswith('MIT License')
print('English documentation, required files, relative links and MIT metadata checks passed.')

spec=importlib.util.spec_from_file_location("app_version",root/"tools/version.py")
version=importlib.util.module_from_spec(spec);spec.loader.exec_module(version)
version.check(root)
print("Application SemVer and lockfiles agree.")
