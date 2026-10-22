"""Recheck downloaded Sony source archives offline; never execute target code.

Run from the repository root. Downloads stay in the ignored research cache.
No archive paths are extracted or followed by this collector.
"""
import base64
import hashlib
import io
import json
from pathlib import Path
import re
import tarfile

ROOT = Path(__file__).resolve().parents[2]
CACHE = ROOT / 'data/firmware-analysis/resident-search-20261019'
OUT = ROOT / 'docs/audits/resident-source-search-2026-10-19'
PINS = {
    'u-boot-1.1.1.local.tgz': ('682f6ae908e1ebfb93ee89ccb81c57ca6895d57eca6c31a8fd9772ab5171de6f', 'Yzr0tMdtfPga8BR2UZ5mBA=='),
    'linux-kernel-2.6.10.local.verified.tgz': ('176e239b7052efbdda8e1beb5eb0baf6b7037cf90ff1d0af5df7b772fec90cab', 'dGrjoG4c3xeun8ikHfHErg=='),
    'mtd-utils-2.6.9_1.1-6.0.0.0501450.src.rpm': ('3ee445de4d2d798493d31a25077b4c2406615692898add78e4fd1e906eaf4875', 'QiZvSLw4mqaJgT2UwQgvJg=='),
}


def collect():
    rows = []
    for name, (sha, md5) in PINS.items():
        data = (CACHE / 'downloads' / name).read_bytes()
        if hashlib.sha256(data).hexdigest() != sha:
            raise ValueError(f'SHA-256 mismatch: {name}')
        if base64.b64encode(hashlib.md5(data).digest()).decode() != md5:
            raise ValueError(f'Content-MD5 mismatch: {name}')
        row = dict(file=name, bytes=len(data), sha256=sha, content_md5_base64=md5)
        if name.endswith('.tgz'):
            with tarfile.open(fileobj=io.BytesIO(data), mode='r:gz') as archive:
                members = archive.getmembers()
                row['archive_members'] = len(members)
                row['installer_name_matches'] = [m.name for m in members if re.search(r'fwpup|linuxrc_fwup', m.name, re.I)]
                hits = []
                scanned = 0
                skipped = []
                for member in members:
                    if not member.isfile():
                        continue
                    if member.size > 4 * 1024 * 1024:
                        skipped.append(member.name)
                        continue
                    scanned += 1
                    payload = archive.extractfile(member).read()
                    terms = [term.decode() for term in (b'fwpup', b'linuxrc_fwup') if term in payload]
                    if terms:
                        hits.append(dict(member=member.name, terms=terms, sha256=hashlib.sha256(payload).hexdigest()))
                row.update(regular_members_scanned=scanned, oversized_regular_members_skipped=skipped, installer_literal_hits=hits)
        rows.append(row)
    # Retained GPL reference files must still equal the upstream member hashes.
    provenance = json.loads((OUT / 'u-boot-provenance.json').read_text())
    for source in provenance['source_files']:
        if hashlib.sha256((OUT / source['file']).read_bytes()).hexdigest() != source['sha256']:
            raise ValueError(f'Retained source mismatch: {source["file"]}')
    report = dict(offline_only=True, target_code_executed=False, archives=rows,
                  retained_u_boot_sources_verified=len(provenance['source_files']),
                  qualification='Source availability is not installed-binary equivalence or flash/recovery qualification.')
    (OUT / 'archive-verification.json').write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
    print(json.dumps(report, indent=2))


if __name__ == '__main__':
    collect()
