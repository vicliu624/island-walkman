"""Synchronize/check application SemVer. Dependencies and historical releases are untouched."""
import argparse,json,os,re,tomllib
from pathlib import Path
ROOT=Path(__file__).resolve().parents[1]
SEMVER=re.compile(r'(0|[1-9]\d*)\.(0|[1-9]\d*)\.(0|[1-9]\d*)(?:-([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?(?:\+([0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*))?\Z')
def valid(v):
 m=SEMVER.fullmatch(v)
 return bool(m) and not any(x.isdigit() and len(x)>1 and x.startswith('0') for x in (m[4]or'').split('.'))
def versions(root=ROOT):
 cargo=tomllib.loads((root/'Cargo.toml').read_text(encoding='utf-8'));lock=tomllib.loads((root/'Cargo.lock').read_text(encoding='utf-8'))
 package=json.loads((root/'ui/package.json').read_text(encoding='utf-8'));npm=json.loads((root/'ui/package-lock.json').read_text(encoding='utf-8'))
 rows=[p['version']for p in lock['package']if p['name']==cargo['package']['name']]
 if len(rows)!=1:raise ValueError('Cargo.lock must contain exactly one root package')
 return {'Cargo.toml':cargo['package']['version'],'Cargo.lock':rows[0],'tauri.conf.json':json.loads((root/'tauri.conf.json').read_text(encoding='utf-8'))['version'],'ui/package.json':package['version'],'ui/package-lock.json':npm['version'],'ui/package-lock.json packages[empty]':npm['packages']['']['version']}
def check(root=ROOT,tag=None):
 values=versions(root);v=values['Cargo.toml']
 if not valid(v):raise ValueError('Invalid SemVer: '+v)
 if any(x!=v for x in values.values()):raise ValueError('Version mismatch: '+json.dumps(values))
 if tag and tag!='v'+v:raise ValueError(f'Tag {tag} does not match v{v}')
 if f'## [{v}]'not in (root/'CHANGELOG.md').read_text(encoding='utf-8'):raise ValueError('Missing current changelog section: '+v)
 for name in ['ui/src/main.tsx','ui/src/desktop-smoke.ts']:
  text=(root/name).read_text(encoding='utf-8')
  if re.search(r'ISLAND WALKMAN · \d+\.\d+\.\d+|关于唱片小岛 · \d+\.\d+\.\d+',text):raise ValueError('Hardcoded UI version: '+name)
 return v
def set_version(v,root=ROOT):
 if not valid(v):raise ValueError('Invalid SemVer: '+v)
 # Stage all changed text before writing, preserving dependency versions.
 changes={};cargo=(root/'Cargo.toml').read_text(encoding='utf-8');changes['Cargo.toml']=re.sub(r'(?m)^version = "[^"]+"',f'version = "{v}"',cargo,count=1)
 lock=(root/'Cargo.lock').read_text(encoding='utf-8');name=tomllib.loads(cargo)['package']['name']
 pat=r'(\[\[package\]\]\nname = "'+re.escape(name)+r'"\nversion = ")[^"]+(")'
 new,n=re.subn(pat,lambda m:m[1]+v+m[2],lock)
 if n!=1:raise ValueError('Root lock entry not unique')
 changes['Cargo.lock']=new
 for path in ['tauri.conf.json','ui/package.json','ui/package-lock.json']:
  data=json.loads((root/path).read_text(encoding='utf-8'));data['version']=v
  if path.endswith('package-lock.json'):data['packages']['']['version']=v
  changes[path]=json.dumps(data,indent=2,ensure_ascii=False)+'\n'
 for path,text in changes.items():(root/path).write_text(text,encoding='utf-8',newline='\n')
if __name__=='__main__':
 p=argparse.ArgumentParser();p.add_argument('--set');p.add_argument('--check',action='store_true');p.add_argument('--tag');a=p.parse_args()
 if a.set:
  set_version(a.set)
  if not a.check and not a.tag:
   print('Set application version to '+a.set+'; add its changelog entry, then run --check.')
   raise SystemExit(0)
 tag=a.tag or (os.environ.get('GITHUB_REF_NAME')if os.environ.get('GITHUB_REF_TYPE')=='tag'else None)
 try:print('Application version consistent: '+check(tag=tag))
 except ValueError as e:raise SystemExit(str(e))
