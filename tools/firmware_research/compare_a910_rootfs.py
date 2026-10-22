"""Offline hash-pinned rootfs inventory; no extraction or target execution."""
import argparse
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile

TERMS = (b'fwpup', b'linuxrc_fwup', b'linuxrc_inst', b'/dev/icx1087_nvp/9',
         b'fWUp', b'fUKt', b'fURt', b'fwup', b'setfuflag', b'nvpflag', b'wrldr')
PINS = ('27efd3993693f28f303842784516824bb327a564fa91cc2d1c86825c7c95b9de',
        'f5ad4b2991ebece66abe6d26ec1f726c56dcd9469ed5888de1e7fd07b11c0557')


def inventory(path, pin):
    raw = path.read_bytes()
    if hashlib.sha256(raw).hexdigest() != pin:
        raise ValueError('Unexpected input hash: ' + str(path))
    entries, payloads, matches = [], {}, []
    with tarfile.open(fileobj=io.BytesIO(raw), mode='r:gz') as archive:
        for member in archive.getmembers():
            row = dict(path=member.name, type=member.type.decode('ascii'), size=member.size,
                       mode=oct(member.mode), uid=member.uid, gid=member.gid)
            if member.issym() or member.islnk():
                row['target'] = member.linkname
            if member.isfile():
                data = archive.extractfile(member).read()
                row['sha256'] = hashlib.sha256(data).hexdigest()
                payloads[member.name] = data
                found = {term.decode(): [m.start() for m in re.finditer(re.escape(term), data)]
                         for term in TERMS if term in data}
                if found:
                    matches.append(dict(path=member.name, sha256=row['sha256'], hits=found))
            entries.append(row)
    if len({row['path'] for row in entries}) != len(entries):
        raise ValueError('Duplicate member paths need explicit ordering analysis')
    return entries, payloads, matches


def compare(old, new, output):
    output.mkdir(parents=True, exist_ok=False)
    older, old_data, old_hits = inventory(old, PINS[0])
    newer, new_data, new_hits = inventory(new, PINS[1])
    old_map = {row['path']: row for row in older}
    new_map = {row['path']: row for row in newer}
    changed = [dict(path=name, old=old_map.get(name), new=new_map.get(name))
               for name in sorted(old_map.keys() | new_map.keys())
               if old_map.get(name) != new_map.get(name)]
    content_changes = [row['path'] for row in changed
                       if (row['old'] or {}).get('sha256') != (row['new'] or {}).get('sha256')]
    mode_changes = [row['path'] for row in changed
                    if (row['old'] or {}).get('mode') != (row['new'] or {}).get('mode')]
    report = dict(offline_only=True, target_code_executed=False,
                  inputs_sha256=dict(rootfs_102=PINS[0], rootfs_103=PINS[1]),
                  members_102=len(older), members_103=len(newer),
                  regular_files_102=len(old_data), regular_files_103=len(new_data),
                  added=sorted(new_map.keys()-old_map.keys()), removed=sorted(old_map.keys()-new_map.keys()),
                  changed=changed, content_changes=content_changes, mode_changes=mode_changes,
                  matches_102=old_hits, matches_103=new_hits,
                  negative_search_limit='All regular member bytes searched for listed literal terms; encoded/dynamic logic is not excluded.',
                  search_terms=[term.decode() for term in TERMS])
    for name, obj in [('rootfs-102-inventory.json', older), ('rootfs-103-inventory.json', newer), ('comparison.json', report)]:
        (output/name).write_text(json.dumps(obj, indent=2)+'\n', encoding='utf-8')
    print(json.dumps(dict(members_102=len(older), members_103=len(newer),
                          changed_entries=len(changed),
                          content_changes=content_changes,
                          match_files_102=[row['path'] for row in old_hits]), indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--old', type=Path, required=True)
    parser.add_argument('--new', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    compare(args.old, args.new, args.output)
