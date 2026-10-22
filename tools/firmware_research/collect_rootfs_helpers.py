"""Bind static wrldr/nvpflag listings to pinned ELF bytes. No target execution."""
import hashlib
import json
from pathlib import Path
import re
import struct
from elf_reader import Elf32

ROOT = Path(__file__).resolve().parents[2]
CACHE = ROOT / 'data/firmware-analysis/resident-search-20261019/rootfs-followup'
OUT = ROOT / 'docs/audits/rootfs-layout-2026-10-19'
PINS = {'wrldr': '6d1b2f2a0d7c336cdcc9a84667561d101cfec12e2ebc878867dc043c49d061a1',
        'nvpflag': '90aa0b11ef01c0997a1db28faf664d3eed931675e7c272b7232d74ed3591e7ac'}


def collect():
    bindings, inputs, listings = {}, {}, []
    for name, pin in PINS.items():
        data = (CACHE/name).read_bytes()
        if hashlib.sha256(data).hexdigest() != pin:
            raise ValueError('Unexpected ELF hash: ' + name)
        inputs[name] = dict(sha256=pin, bytes=len(data))
        elf = Elf32(data)
        addresses = set()
        for path in sorted((OUT/'decompilation'/name).glob('*.txt')):
            payload = path.read_bytes()
            if b'// INCOMPLETE:' in payload:
                raise ValueError('Incomplete decompilation: ' + str(path))
            listings.append(dict(file=path.relative_to(OUT).as_posix(), sha256=hashlib.sha256(payload).hexdigest()))
            addresses.update(int(at, 16) for at in re.findall(r'DAT_([0-9a-f]{8})', payload.decode()))
        rows = []
        for address in sorted(addresses):
            word = struct.unpack('<I', elf.read_va(address, 4))[0]
            row = dict(va=hex(address), word=hex(word))
            try:
                text = bytearray()
                for index in range(400):
                    value = elf.read_va(word+index, 1)[0]
                    if not value:
                        break
                    text.append(value)
                if text and len(text) < 400 and all(v in (9, 10, 13) or 32 <= v < 127 for v in text):
                    row['pointed_ascii'] = text.decode('ascii')
            except ValueError:
                pass  # BSS/global pointer or a non-string constant, not missing evidence.
            rows.append(row)
        bindings[name] = rows
    (OUT/'elf-data-bindings.json').write_text(json.dumps(bindings, indent=2)+'\n', encoding='utf-8')
    (OUT/'helper-provenance.json').write_text(json.dumps(dict(offline_only=True,
        target_code_executed=False, inputs=inputs, listings=listings,
        limits='Static decompiler listings and literal bindings, not execution, driver verification or a proven resident-install caller.'), indent=2)+'\n', encoding='utf-8')
    print('Verified 2 pinned ELF inputs and bound', len(listings), 'static function listings.')


if __name__ == '__main__':
    collect()
