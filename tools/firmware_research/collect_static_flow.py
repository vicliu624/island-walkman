"""Read pinned PE/INI inputs and export static instructions. Never load a DLL.

The only child process is llvm-objdump, with the Sony file as an input operand.
Output must be a new directory so prior audit evidence is never overwritten.
"""
import argparse
import configparser
import hashlib
import json
from pathlib import Path
import struct
import subprocess

PINNED_EXE = '3cf7eb2e354d2e6d3f0cddd06ab67755696579a0d2cc85b68672ecc44ccb3c1c'
PINNED_DLL = '2b7550cd945a8d71d139063fce0ce006bfa6b59ee940f17e5151bdb6e35ebe2f'
PINNED_RES = '0536de4cc4373f2f1391fbb52f11edf47af28fa52551959d569b6488c639b243'


class PE32:
    def __init__(self, data):
        self.data = data
        if data[:2] != b'MZ':
            raise ValueError('Not PE')
        nt = struct.unpack_from('<I', data, 0x3c)[0]
        if data[nt:nt + 4] != b'PE\0\0' or struct.unpack_from('<H', data, nt + 24)[0] != 0x10b:
            raise ValueError('Not PE32')
        self.base = struct.unpack_from('<I', data, nt + 52)[0]
        table = nt + 24 + struct.unpack_from('<H', data, nt + 20)[0]
        self.sections = []
        for i in range(struct.unpack_from('<H', data, nt + 6)[0]):
            virtual_size, rva, raw_size, raw = struct.unpack_from('<IIII', data, table + i * 40 + 8)
            self.sections.append((rva, virtual_size, raw_size, raw))

    def read(self, va, size):
        rva = va - self.base
        for start, _, raw_size, raw in self.sections:
            delta = rva - start
            if 0 <= delta and delta + size <= raw_size and raw + delta + size <= len(self.data):
                return self.data[raw + delta:raw + delta + size]
        raise ValueError('Address outside file-backed PE section')

    def words(self, va, count):
        return struct.unpack('<' + 'I' * count, self.read(va, count * 4))


def digest(data):
    return hashlib.sha256(data).hexdigest()


def version_resource_strings(data):
    if digest(data) != PINNED_RES:
        raise ValueError('Resource DLL hash mismatch')
    pe = PE32(data)
    nt = struct.unpack_from('<I', data, 0x3c)[0]
    rva, _ = struct.unpack_from('<II', data, nt + 24 + 96 + 16)
    base = pe.base + rva

    def directory(offset):
        named, count = struct.unpack_from('<HH', pe.read(base + offset, 16), 12)
        return [struct.unpack('<II', pe.read(base + offset + 16 + i * 8, 8)) for i in range(named + count)]

    strings = {}
    for kind, first in directory(0):
        if kind != 6:
            continue
        for block, second in directory(first & 0x7fffffff):
            if block != 13:
                continue
            for language, leaf in directory(second & 0x7fffffff):
                if language != 0x409:
                    continue
                at, length, _, _ = struct.unpack('<IIII', pe.read(base + leaf, 16))
                raw, pos = pe.read(pe.base + at, length), 0
                for i in range(16):
                    length = struct.unpack_from('<H', raw, pos)[0]
                    pos += 2
                    text = raw[pos:pos + length * 2].decode('utf-16le')
                    pos += length * 2
                    if (block - 1) * 16 + i in (202, 203, 204):
                        strings[str((block - 1) * 16 + i)] = text
    if set(strings) != {'202', '203', '204'}:
        raise ValueError('Version confirmation resources missing')
    return strings


def collect(exe_path, dll_path, ini_path, objdump, output, resource_path=None):
    exe_data, dll_data = exe_path.read_bytes(), dll_path.read_bytes()
    if digest(exe_data) != PINNED_EXE or digest(dll_data) != PINNED_DLL:
        raise ValueError('Original binary hash mismatch; stop rather than reuse address assumptions')
    if objdump.name.lower() not in ('llvm-objdump', 'llvm-objdump.exe'):
        raise ValueError('Only llvm-objdump is accepted as the analysis executable')
    version_strings = version_resource_strings(resource_path.read_bytes()) if resource_path else {}
    exe, dll = PE32(exe_data), PE32(dll_data)
    ini = configparser.ConfigParser(interpolation=None)
    ini.read_string(ini_path.read_text(encoding='ascii'))
    devices = []
    for i, model in enumerate(['NW-A916', 'NW-A918', 'NW-A919']):
        cfg = dict(ini[f'DEVICE{i:02d}'])
        expected = {'devicename':model, 'vid':'0x054c', 'pid':'0x032A', 'classifytype':'5',
                    'commandtype':'0', 'signature':'dbmn', 'version00':'1.03',
                    'fwpackage00':'NW_A91X.UPG', 'fwpackagewritepathname':r'\NW_A91X.UPG', 'timer':'1:40'}
        if any(cfg.get(k) != v for k, v in expected.items()):
            raise ValueError('Unexpected official device configuration')
        devices.append(cfg)
    bindings = []
    for name, iid_va, com_map, offset, vtable, count in [
        ('main',0x41c38c,0x1002f7a8,0,0x1002f774,10),
        ('updater',0x41c39c,0x1002f7b4,4,0x1002f74c,9),
        ('transfer',0x41c3ac,0x1002f7c0,8,0x1002f738,4),
        ('device',0x41c3bc,0x1002f7cc,12,0x1002f704,12),
    ]:
        iid, object_offset, discriminator = dll.words(com_map, 3)
        if dll.read(iid, 16) != exe.read(iid_va, 16) or object_offset != offset or discriminator != 1:
            raise ValueError('COM interface binding mismatch')
        bindings.append({'interface':name,'exe_iid_va':hex(iid_va),'iid_bytes':exe.read(iid_va,16).hex(),
                         'dll_com_map_va':hex(com_map),'object_offset':offset,'vtable':hex(vtable),
                         'slots':{hex(i*4):hex(v)for i,v in enumerate(dll.words(vtable,count))}})
    constructor = bytes.fromhex('c70674f70210c746044cf70210c7460838f70210c7460c04f70210')
    if dll.read(0x10006ec3,len(constructor)) != constructor:
        raise ValueError('Constructor/vtable assignment mismatch')
    output.mkdir(parents=True, exist_ok=False)
    ranges = [
        ('exe-selection',exe_path,0x404d90,0x40570d),
        ('exe-version-choice',exe_path,0x40798d,0x4079e4),
        ('exe-target-selection',exe_path,0x403eb0,0x403f59),
        ('dll-open-volume',dll_path,0x100010e0,0x10001179),
        ('exe-timers',exe_path,0x405cf0,0x406650),
        ('exe-dispatch',exe_path,0x408150,0x408450),
        ('exe-worker',exe_path,0x408450,0x4089c0),
        ('exe-snapshot',exe_path,0x407d40,0x407ed6),
        ('dll-constructor',dll_path,0x10006eb0,0x10006f10),
        ('dll-transfer-wrapper',dll_path,0x1000b600,0x1000b6dd),
        ('dll-file-upload',dll_path,0x10009100,0x10009640),
        ('dll-copy-error-tail',dll_path,0x10007a10,0x10007dd1),
        ('dll-cleanup-wrapper',dll_path,0x1000b6e0,0x1000b79b),
        ('dll-delete',dll_path,0x10009920,0x10009a19),
        ('dll-spti',dll_path,0x10001180,0x10001306),
    ]
    exports = []
    for name, binary, start, end in ranges:
        result = subprocess.run([str(objdump),'-d',f'--start-address={hex(start)}',f'--stop-address={hex(end)}',str(binary)],
                                capture_output=True,text=True,encoding='utf-8',check=True,timeout=30)
        data = result.stdout.encode('utf-8')
        path = output / (name + '.txt')
        path.write_bytes(data)
        exports.append({'file':path.name,'input':binary.name,'start_va':hex(start),'end_va_exclusive':hex(end),'sha256':digest(data)})
    facts = {'evidence_kind':'static_binary_analysis_not_device_trace','sony_binary_executed':False,
             'physical_device_access':False,'version_confirmations_en_US':version_strings,
             'official_resource_sha256':PINNED_RES if resource_path else None,'official_exe_sha256':PINNED_EXE,'official_dll_sha256':PINNED_DLL,
             'ini_sha256':digest(ini_path.read_bytes()),'device_sections':devices,'com_bindings':bindings,
             'constructor_bytes_verified':True,'timer_seconds':100,'timeout_seconds_when_absent':150,
             'copy_error_map':{'112':'0x80047008','5':'0x80047007','other':'0x80047006'},
             'activation_false_result_accepted_packed_status':'0xff020400',
             'activation_false_result_mask':'0x00ff0000',
             'model_limitations':['Not a CPU execution or hardware transcript','Specific pinned MSC file-upload branch',
                                  'Directory/MTP/command-type-1 and Windows Me branches not fully simulated',
                                  'Resident installer and physical recovery remain unverified'],
             'disassembly':exports}
    (output/'facts.json').write_text(json.dumps(facts,indent=2)+'\n',encoding='utf-8')
    return facts


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--exe', type=Path, required=True)
    parser.add_argument('--dll', type=Path, required=True)
    parser.add_argument('--ini', type=Path, required=True)
    parser.add_argument('--resources', type=Path)
    parser.add_argument('--objdump', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    facts = collect(args.exe.resolve(strict=True),args.dll.resolve(strict=True),args.ini.resolve(strict=True),
                    args.objdump.resolve(strict=True),args.output.resolve(),
                    args.resources.resolve(strict=True) if args.resources else None)
    print(json.dumps({'verified_interfaces':len(facts['com_bindings']),'disassembly_files':len(facts['disassembly']),
                      'physical_device_access':False,'output':str(args.output)},indent=2))
