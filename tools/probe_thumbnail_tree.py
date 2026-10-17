"""Controlled A918 experiment: change only the existing 2D tree title marker.

Firmware evidence: docs/research/firmware/rea/artwork-tree-selection.json,
oma-backend-initialize.json and tree-marker-match.json. Not a general writer.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import time

from install_cover_probe import files, sha


def patch(blob):
    if len(blob) != 6528 or blob[:4] != b'GTIF':
        raise RuntimeError('Expected the observed A918 45-record tree info table')
    start = int.from_bytes(blob[20:24], 'big')
    if start != 32 or blob[start:start+8] != b'GTFB\x00\x2d\x00\x90':
        raise RuntimeError('Unexpected tree info class/count/stride')
    slot = start + 16 + 44*144 + 16
    if blob[slot:slot+6] != b'TIT2\x00\x02':
        raise RuntimeError('Expected existing tree 2D title')
    old = 'STD_TPE1'.encode('utf-16-be')
    if blob[slot+6:slot+128] != old + bytes(122-len(old)):
        raise RuntimeError('Tree 2D has a different marker; refuse replacement')
    new = 'STD_TPE1_THUMBNAIL'.encode('utf-16-be')
    result = bytearray(blob)
    result[slot+6:slot+128] = new + bytes(122-len(new))
    assert len(result) == len(blob)
    assert result[:slot+6] == blob[:slot+6] and result[slot+128:] == blob[slot+128:]
    return bytes(result)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--install', action='store_true')
    args = parser.parse_args()
    project = Path(__file__).resolve().parents[1]
    baseline = project/'data/device-observations/post-firmware-1792248821086586700/comparison.json'
    expected = json.loads(baseline.read_text())['manifest']
    library = Path('F:/OMGAUDIO')
    if os.name != 'nt' or not library.is_dir():
        raise RuntimeError('Observed Windows A918 library is unavailable')
    before = {k:v[1] for k,v in files(library).items()}
    if before != {k.upper():v for k,v in expected.items()}:
        raise RuntimeError('Device differs from post-firmware baseline')
    table = library/'02TREINF.DAT'
    original = table.read_bytes()
    candidate = patch(original)
    run = project/'data/cover-installs'/('thumbnail-tree-'+str(time.time_ns()))
    run.mkdir(parents=True)
    (run/'02TREINF.candidate.DAT').write_bytes(candidate)
    report = dict(device=str(library), changed=['02TREINF.DAT'],
                  before=before, changed_bytes=sum(a!=b for a,b in zip(original,candidate)),
                  marker_before='STD_TPE1', marker_after='STD_TPE1_THUMBNAIL',
                  installed=False, playback_verified=False, cover_verified=False)
    if args.install:
        backup = run/'before/OMGAUDIO'
        shutil.copytree(library,backup)
        if {k:v[1] for k,v in files(backup).items()} != before:
            raise RuntimeError('Backup hash verification failed')
        if {k:v[1] for k,v in files(library).items()} != before:
            raise RuntimeError('Device changed before commit')
        temp = library/'ISLAND-THUMBNAIL.TMP'
        with temp.open('xb') as f:
            f.write(candidate)
            f.flush()
            os.fsync(f.fileno())
        candidate_hash = hashlib.sha256(candidate).hexdigest()
        if sha(temp) != candidate_hash:
            raise RuntimeError('Staged table verification failed')
        os.replace(temp,table)
        after = {k:v[1] for k,v in files(library).items()}
        expected_after = dict(before)
        expected_after['02TREINF.DAT'] = candidate_hash
        if after != expected_after:
            raise RuntimeError('Final device hash mismatch; backup is retained')
        report.update(installed=True, backup=str(backup), after=after, audio_unchanged=True)
    (run/'install.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
    print(json.dumps(dict(report_path=str(run/'install.json'),installed=report['installed'],
                         changed_bytes=report['changed_bytes']),indent=2))


if __name__ == '__main__':
    main()
