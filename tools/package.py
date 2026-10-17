"""Build portable archives and native packages from an already tested binary."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import plistlib
import shutil
import subprocess
import tarfile
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def copy(source, target):
    target.parent.mkdir(parents=True, exist_ok=True)
    if source.is_dir():
        shutil.copytree(source, target)
    else:
        shutil.copy2(source, target)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--platform', required=True, choices=['windows', 'debian', 'macos'])
    parser.add_argument('--arch', required=True, choices=['x86_64', 'aarch64'])
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, default=ROOT/'dist')
    args = parser.parse_args()
    version = next(line.split('"')[1] for line in (ROOT/'Cargo.toml').read_text().splitlines() if line.startswith('version ='))
    name = f'island-walkman-{version}-{args.platform}-{args.arch}'
    dist = args.output.resolve()
    stage = dist/'stage'/name
    if stage.exists():
        raise SystemExit(f'Refusing to overwrite an existing stage: {stage}')
    stage.mkdir(parents=True)
    ui = ROOT/'ui/dist'
    if not (ui/'index.html').is_file():
        raise SystemExit('Build the UI before packaging.')
    if args.platform == 'macos':
        app = stage/'Island Walkman.app'
        executable = app/'Contents/MacOS/island-walkman'
        copy(args.binary, executable)
        copy(ui, app/'Contents/Resources/ui')
        plist = dict(CFBundleName='Island Walkman', CFBundleDisplayName='Island Walkman',
                     CFBundleIdentifier='io.github.island-walkman.app', CFBundleVersion=version,
                     CFBundleShortVersionString=version, CFBundleExecutable='island-walkman',
                     CFBundlePackageType='APPL', LSMinimumSystemVersion='14.0', NSHighResolutionCapable=True)
        (app/'Contents/Info.plist').write_bytes(plistlib.dumps(plist))
        executable.chmod(0o755)
        subprocess.run(['codesign', '--force', '--sign', '-', str(app)], check=True)
        subprocess.run(['codesign', '--verify', '--deep', '--strict', str(app)], check=True)
    else:
        executable = stage/('island-walkman.exe' if args.platform == 'windows' else 'island-walkman')
        copy(args.binary, executable)
        copy(ui, stage/'ui')
        if args.platform != 'windows':
            executable.chmod(0o755)
    for file in ['README.md', 'CHANGELOG.md', 'LICENSE', 'THIRD_PARTY_NOTICES.md']:
        copy(ROOT/file, stage/file)
    # Preserve licenses supplied with bundled UI dependencies, including fonts.
    notices = stage/'third-party-licenses'
    copy(ROOT/'licenses',notices/'fonts')
    lock=json.loads((ROOT/'ui/package-lock.json').read_text())
    for location,metadata in lock['packages'].items():
        if not location or metadata.get('dev'):
            continue
        package=location.removeprefix('node_modules/')
        package_root = ROOT/'ui'/location
        for pattern in ['LICENSE*', 'OFL*', 'COPYING*']:
            for source in package_root.rglob(pattern):
                if source.is_file():
                    copy(source, notices/package/source.relative_to(package_root))
    cargo=json.loads(subprocess.check_output(['cargo','metadata','--locked','--format-version','1'],cwd=ROOT))
    rust_packages=[]
    for package in cargo['packages']:
        rust_packages.append({key:package.get(key) for key in ['name','version','license','repository']})
        source_root=Path(package['manifest_path']).parent
        if source_root==ROOT:
            continue
        for pattern in ['LICENSE*','COPYING*']:
            for source in source_root.glob(pattern):
                if source.is_file():
                    copy(source,notices/'rust'/(package['name']+'-'+package['version'])/source.name)
    (notices/'rust-packages.json').write_text(json.dumps(rust_packages,indent=2))
    artifacts = []
    if args.platform == 'windows':
        archive = dist/(name+'.zip')
        with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as output:
            for file in sorted(stage.rglob('*')):
                if file.is_file():
                    output.write(file, file.relative_to(stage.parent))
        artifacts.append(archive)
    else:
        archive = dist/(name+'.tar.gz')
        with tarfile.open(archive, 'w:gz') as output:
            output.add(stage, arcname=name)
        artifacts.append(archive)
    if args.platform == 'debian':
        debroot = dist/'stage'/(name+'-deb')
        appdir = debroot/'usr/lib/island-walkman'
        copy(stage, appdir)
        launcher = debroot/'usr/bin/island-walkman'
        launcher.parent.mkdir(parents=True, exist_ok=True)
        launcher.write_text('#!/bin/sh\nexec /usr/lib/island-walkman/island-walkman "$@"\n')
        launcher.chmod(0o755)
        desktop = debroot/'usr/share/applications/island-walkman.desktop'
        desktop.parent.mkdir(parents=True)
        desktop.write_text('[Desktop Entry]\nType=Application\nName=Island Walkman\nComment=Local music library and Sony WALKMAN transfer tool\nExec=island-walkman\nTerminal=false\nCategories=AudioVideo;Audio;\n')
        control = debroot/'DEBIAN/control'
        control.parent.mkdir()
        control.write_text(f'Package: island-walkman\nVersion: {version}\nArchitecture: {"amd64" if args.arch == "x86_64" else "arm64"}\nMaintainer: Island Walkman contributors\nSection: sound\nPriority: optional\nRecommends: xdg-utils\nDescription: Local music manager for Sony WALKMAN\n Rust core with a local browser interface.\n')
        deb = dist/(name+'.deb')
        subprocess.run(['dpkg-deb', '--root-owner-group', '--build', str(debroot), str(deb)], check=True)
        artifacts.append(deb)
    sums = dist/(name+'.sha256')
    sums.write_text(''.join(f'{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n' for p in artifacts))
    (dist/(name+'.json')).write_text(json.dumps(dict(version=version, platform=args.platform,
        arch=args.arch, executable=str(executable), artifacts=[str(p) for p in artifacts]), indent=2))
    print(executable)


if __name__ == '__main__':
    main()
