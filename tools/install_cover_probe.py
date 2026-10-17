"""Install an audited Rust cover-only probe; never rebuild or replace audio.

This research helper is separate from the application's gated transfer API.
It requires an exact current-library hash match and retains a verified backup.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import time
import uuid


def sha(path):
    with path.open('rb') as file:
        return hashlib.file_digest(file, 'sha256').hexdigest()


def files(root):
    result = {}
    for p in root.rglob('*'):
        if p.is_symlink():
            raise RuntimeError('Refusing symlink in library')
        if p.is_file():
            key = p.relative_to(root).as_posix().upper()
            if key in result:
                raise RuntimeError('Case-colliding filenames')
            result[key] = (p, sha(p))
    return result


def hierarchy_records(blob):
    if len(blob) < 48 or blob[:4] != b'GPIF' or blob[8] != 1:
        raise RuntimeError('Expected single-section hierarchy table')
    start = int.from_bytes(blob[20:24], 'big')
    length = int.from_bytes(blob[24:28], 'big')
    if start < 32 or start + length > len(blob) or blob[start:start+4] != b'GPFB':
        raise RuntimeError('Invalid hierarchy section bounds')
    count = int.from_bytes(blob[start+4:start+6], 'big')
    stride = int.from_bytes(blob[start+6:start+8], 'big')
    if stride < 16 or 16 + count*stride != length:
        raise RuntimeError('Invalid hierarchy record lengths')
    records = []
    for i in range(count):
        record = blob[start+16+i*stride:start+16+(i+1)*stride]
        fields = int.from_bytes(record[12:14], 'big')
        if record[14:16] != b'\x00\x80' or 16 + fields*128 != stride:
            raise RuntimeError('Invalid hierarchy field length')
        tags = {}
        for j in range(fields):
            slot = record[16+j*128:16+(j+1)*128]
            if slot[:4] in tags:
                raise RuntimeError('Duplicate hierarchy field')
            tags[slot[:4]] = slot
        records.append((record[:12], tags))
    return start, stride, records


def validate_hierarchy_patch(before, after):
    old_start, old_stride, old = hierarchy_records(before)
    new_start, new_stride, new = hierarchy_records(after)
    if old_start != new_start or len(old) != len(new) or new_stride != old_stride + 256:
        raise RuntimeError('Unexpected hierarchy structural change')
    old_header = bytearray(before[:old_start+16])
    new_header = bytearray(after[:new_start+16])
    for a,b in [(24,28),(old_start+6,old_start+8)]:
        old_header[a:b] = new_header[a:b]
    if old_header != new_header:
        raise RuntimeError('Hierarchy header changes exceed lengths')
    primary_count = 0
    for (old_identity, old_fields), (new_identity, new_fields) in zip(old,new):
        if old_identity != new_identity or set(old_fields) != {b'TIT2',b'XSOT'} or set(new_fields) != {b'TIT2',b'XSOT',b'PICP',b'PIC0'}:
            raise RuntimeError('Hierarchy record identity/fields changed')
        if any(new_fields[tag] != slot for tag,slot in old_fields.items()):
            raise RuntimeError('Original hierarchy metadata changed')
        for tag in [b'PICP',b'PIC0']:
            if new_fields[tag][4:6] != b'\x00\x02' or new_fields[tag][-2:] != b'\x00\x00':
                raise RuntimeError('Invalid hierarchy cover text encoding')
        primary_count += bool(new_fields[b'PICP'][6:8] != b'\x00\x00')
    if primary_count != 1:
        raise RuntimeError('Expected exactly one album cover in hierarchy candidate')


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('device', type=Path)
    parser.add_argument('probe', type=Path)
    args = parser.parse_args()
    root = args.device.resolve(strict=True)
    if os.name != 'nt' or root.drive.upper() == os.environ.get('SystemDrive', 'C:').upper():
        parser.error('This controlled validation helper requires a Windows removable device')
    if not all((root / p).is_dir() for p in ('DCIM', 'VIDEO', 'PICTURE', 'OMGAUDIO')):
        parser.error('Expected Walkman root directories are missing')
    library = root / 'OMGAUDIO'
    probe = args.probe.resolve(strict=True)
    report = json.loads((probe / 'probe.json').read_text(encoding='utf-8'))
    if Path(report['source']).resolve() != library:
        parser.error('Probe was not prepared from this device')
    if report['changed'] not in (['03GINF01.DAT'], ['03GINF2D.DAT']):
        parser.error('Only an audited single cover-table patch is allowed')
    table_name = report['changed'][0]
    source = probe / 'OMGAUDIO'
    actual = files(library)
    expected = {k.upper(): v for k, v in report['before'].items()}
    if {k: v[1] for k, v in actual.items()} != expected:
        raise RuntimeError('Device library changed after preparation; prepare a fresh probe')
    staged = files(source)
    if {k: v[1] for k, v in staged.items()} != {k.upper(): v for k, v in report['after'].items()}:
        raise RuntimeError('Prepared files changed after verification')
    additions = set(staged) - set(actual)
    if len(additions) != 3 or any(not p.startswith('20P') or not p.endswith('.JPG') for p in additions):
        raise RuntimeError('Expected exactly three new cover JPEGs')
    table, old_hash = actual[table_name]
    before = table.read_bytes()
    after = staged[table_name][0].read_bytes()
    if before[:4] != b'GPIF':
        raise RuntimeError('Table layout changed')
    hierarchy = table_name == '03GINF2D.DAT'
    if hierarchy:
        validate_hierarchy_patch(before,after)
    elif len(before) != len(after):
        raise RuntimeError('Album patch must retain table size')
    class_start = int.from_bytes(before[20:24], 'big')
    if before[class_start:class_start + 4] != b'GPFB':
        raise RuntimeError('Expected GPFB class')
    count = int.from_bytes(before[class_start + 4:class_start + 6], 'big')
    stride = int.from_bytes(before[class_start + 6:class_start + 8], 'big')
    allowed = set()
    for record in range(count):
        start = class_start + 16 + record * stride
        field_count = int.from_bytes(before[start + 12:start + 14], 'big')
        if start + stride > len(before) or 16 + field_count * 128 > stride:
            raise RuntimeError('Invalid GPFB record bounds')
        for field in range(field_count):
            offset = start + 16 + field * 128
            if before[offset:offset + 4] in (b'PICP', b'PIC0'):
                allowed.update(range(offset + 6, offset + 128))
    changed = {i for i, (a, b) in enumerate(zip(before, after)) if a != b}
    if not changed or (not hierarchy and not changed.issubset(allowed)):
        raise RuntimeError('Patch changes bytes outside existing cover text payloads')
    project = Path(__file__).resolve().parents[1]
    run = project / 'data' / 'cover-installs' / (str(time.time_ns()) + '-' + uuid.uuid4().hex[:8])
    backup = run / 'before' / 'OMGAUDIO'
    shutil.copytree(library, backup)
    if {k: v[1] for k, v in files(backup).items()} != expected:
        raise RuntimeError('Backup verification failed')
    for name in sorted(additions):
        src = staged[name][0]
        destination = library / src.relative_to(source)
        destination.parent.mkdir(parents=True, exist_ok=True)
        with destination.open('xb') as output, src.open('rb') as input_file:
            shutil.copyfileobj(input_file, output)
            output.flush()
            os.fsync(output.fileno())
        if sha(destination) != staged[name][1]:
            raise RuntimeError('Device cover verification failed')
    if sha(table) != old_hash:
        raise RuntimeError('Album table changed before commit')
    temp = table.with_name('ISLAND-COVER-' + uuid.uuid4().hex + '.TMP')
    with temp.open('xb') as output:
        output.write(after)
        output.flush()
        os.fsync(output.fileno())
    if sha(temp) != staged[table_name][1]:
        raise RuntimeError('Staged table verification failed')
    os.replace(temp, table)
    final = {k: v[1] for k, v in files(library).items()}
    if final != {k: v[1] for k, v in staged.items()}:
        raise RuntimeError('Final library verification failed; backup retained at ' + str(backup))
    result = {'probe': str(probe), 'device': str(root), 'backup': str(backup),
              'changed_table': str(table), 'changed_bytes': len(changed),
              'table_size_before': len(before), 'table_size_after': len(after),
              'new_files': sorted(additions), 'audio_unchanged': True,
              'device_playback_verified': False, 'device_cover_verified': False}
    (run / 'install.json').write_text(json.dumps(result, ensure_ascii=False, indent=2), encoding='utf-8')
    print(json.dumps(result, ensure_ascii=False, indent=2))


if __name__ == '__main__':
    main()
