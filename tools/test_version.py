import importlib.util,json,shutil,tempfile,tomllib,unittest
from pathlib import Path
spec=importlib.util.spec_from_file_location('version',Path(__file__).with_name('version.py'));v=importlib.util.module_from_spec(spec);spec.loader.exec_module(v)
class VersionTests(unittest.TestCase):
 def fixture(self):
  folder=tempfile.TemporaryDirectory();self.addCleanup(folder.cleanup);root=Path(folder.name)
  for n in ['Cargo.toml','Cargo.lock','tauri.conf.json','ui/package.json','ui/package-lock.json','CHANGELOG.md','ui/src/main.tsx','ui/src/desktop-smoke.ts']:
   dst=root/n;dst.parent.mkdir(parents=True,exist_ok=True);shutil.copyfile(v.ROOT/n,dst)
  return root
 def test_current_manifests_and_prerelease_tag_match(self):
  current=v.versions()['Cargo.toml'];self.assertEqual(v.check(self.fixture(),tag='v'+current),current)
 def test_mismatch_prevents_build(self):
  root=self.fixture();p=root/'tauri.conf.json';data=json.loads(p.read_text());data['version']='0.3.0';p.write_text(json.dumps(data))
  with self.assertRaisesRegex(ValueError,'Version mismatch'):v.check(root)
 def test_stable_tag_cannot_publish_prerelease_code(self):
  with self.assertRaisesRegex(ValueError,'does not match'):v.check(self.fixture(),tag='v999.0.0')
 def test_invalid_semver_is_rejected(self):
  for bad in ['01.4.0','0.4','0.4.0-alpha.01','v0.4.0','0.4.0-']:self.assertFalse(v.valid(bad),bad)
 def test_version_change_preserves_all_dependency_versions(self):
  root=self.fixture();before=tomllib.loads((root/'Cargo.lock').read_text())['package'];npm=json.loads((root/'ui/package-lock.json').read_text());v.set_version('0.4.0-alpha.2',root)
  after=tomllib.loads((root/'Cargo.lock').read_text())['package'];self.assertEqual([p for p in before if p['name']!='island-walkman'],[p for p in after if p['name']!='island-walkman'])
  updated=json.loads((root/'ui/package-lock.json').read_text());npm['version']=updated['version'];npm['packages']['']['version']=updated['packages']['']['version'];self.assertEqual(npm,updated)
if __name__=='__main__':unittest.main()
