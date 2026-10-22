"""Collect hash-pinned device-install evidence without executing target code.

Parses archive/ext2/ELF bytes in memory. Optional llvm-objdump and 7z are invoked
only as static readers. Never mounts/extracts an image, opens a device or loads a
Sony library. Output directory must be new.
"""
import argparse
import gzip
import hashlib
import io
import json
from pathlib import Path
import re
import struct
import subprocess
import tarfile
from elf_reader import Elf32
from ext2_reader import Ext2Image

PINS = {
    'decrypted-0.bin':'60aedd9213cd6178edcc92a1f168d3e9efda3d7af21292c7d3ea8bbceae713d4',
    'decrypted-1.bin':'680bf2c4cae6be0d7d3f7028c150f45a3648a82feb0231772a9e60b33262faf4',
    'decrypted-2.bin':'4c11d0c396dff18465ae1d24aa0692edd5ecb14ca5790232f7fdba92c8ae4252',
    'decrypted-3.bin':'f5ad4b2991ebece66abe6d26ec1f726c56dcd9469ed5888de1e7fd07b11c0557',
    'kernel-recovered.elf':'fe5e012f08f3df51bcf0cf0503d0345658225e151f9922612a05f7d8bbcf5752',
}
KEYWORDS = (b'fwpup', b'fWUp', b'/dev/icx1087_nvp/9')


def sha(data):
    return hashlib.sha256(data).hexdigest()


def hits(data):
    return {needle.decode('ascii'):[match.start() for match in re.finditer(re.escape(needle),data)]
            for needle in KEYWORDS if needle in data}


def newc_entries(data):
    output, offset = [], 0
    while offset + 110 <= len(data):
        header = data[offset:offset + 110]
        if header[:6] not in (b'070701',b'070702'):
            raise ValueError('Invalid embedded newc archive')
        fields = [int(header[6 + i * 8:14 + i * 8],16) for i in range(13)]
        size, name_size = fields[6], fields[11]
        name = data[offset + 110:offset + 110 + name_size]
        if not name.endswith(b'\0'):
            raise ValueError('Invalid newc name')
        name = name[:-1].decode('ascii')
        offset = (offset + 110 + name_size + 3) & ~3
        if name == 'TRAILER!!!':
            return output
        if offset + size > len(data):
            raise ValueError('Truncated newc file')
        output.append({'path':name,'mode':oct(fields[1]),'size':size,'sha256':sha(data[offset:offset+size])})
        offset = (offset + size + 3) & ~3
    raise ValueError('Missing newc trailer')


def collect(work, output, objdump=None, sevenzip=None):
    inputs = {}
    for name, expected in PINS.items():
        data = (work/name).read_bytes()
        if sha(data) != expected:
            raise ValueError('Input hash mismatch: ' + name)
        inputs[name] = data
    root_entries, root_data, searches, importers, unparsable = [], {}, [], [], []
    with tarfile.open(fileobj=io.BytesIO(inputs['decrypted-3.bin']),mode='r:gz') as archive:
        for member in archive.getmembers():
            row = {'path':member.name,'type':member.type.decode('ascii'),'size':member.size}
            if member.issym() or member.islnk():
                row['target'] = member.linkname
            if member.isfile():
                data = archive.extractfile(member).read()
                row['sha256'] = sha(data)
                root_data[member.name] = data
                found = hits(data)
                if found:
                    searches.append({'path':member.name,'hits':found,'sha256':sha(data)})
                if data.startswith(b'\x7fELF'):
                    try:
                        elf = Elf32(data)
                        imported = sorted({symbol['name'] for symbol in elf.symbols if symbol['section'] == 0
                                           and symbol['name'].startswith('icx1087_nvp_')})
                        if imported:
                            importers.append({'path':member.name,'imports':imported})
                    except (ValueError,struct.error) as error:
                        unparsable.append({'path':member.name,'reason':str(error)})
            root_entries.append(row)
    if len(root_entries) != 613:
        raise ValueError('Unexpected rootfs inventory size')
    initrd = Ext2Image(inputs['decrypted-2.bin'])
    init_entries, init_data = [], {}
    for entry in initrd.entries():
        row = dict(entry)
        if entry['kind'] == 0x8000:
            data = initrd.contents(entry['inode'])
            row['sha256'] = sha(data)
            init_data[entry['path']] = data
            if hits(data):
                row['keyword_hits'] = hits(data)
        init_entries.append(row)
    module_bytes = root_data['./lib/modules/icx1087_nvp.ko']
    if module_bytes != init_data['/lib/modules/icx1087_nvp.ko']:
        raise ValueError('Root/initrd NVP drivers differ')
    if sha(module_bytes) != 'c0639bc1f31c0208ee71381c11777950d5af157baf23bde6ecde6cbd148ab377':
        raise ValueError('NVP driver hash mismatch')
    module = Elf32(module_bytes)
    node = struct.unpack_from('<II',module.symbol_bytes('NodeInfo'),9*8)
    area = module.symbol('Area')
    target_section, target_offset = module.absolute_relocation('.data',area['value'] + 20)
    table = module.symbol('MSCZone')
    if node != (0,8) or target_section != '.rodata' or target_offset != table['value']:
        raise ValueError('NVP node/relocation binding differs')
    field = struct.unpack_from('<II',module.symbol_bytes('MSCZone'),8*8)
    if field != (2044,4):
        raise ValueError('Unexpected logical flag field')
    # The recovered ELF is a research reconstruction, bound back to raw kernel.
    kernel = Elf32(inputs['kernel-recovered.elf'])
    kernel_bindings = []
    for start, end in [(0xc0145544,0xc01459f4),(0xc01459f4,0xc0145eb4),
                       (0xc0145eb4,0xc01462ac),(0xc014756c,0xc0147b98)]:
        data = kernel.read_va(start,end-start)
        if data != inputs['decrypted-1.bin'][start-0xc0008000:end-0xc0008000]:
            raise ValueError('Recovered kernel differs from pinned raw image')
        kernel_bindings.append({'start_va':hex(start),'end_va_exclusive':hex(end),'raw_image_offset':hex(start-0xc0008000),'sha256':sha(data)})
    compressed = inputs['decrypted-1.bin'][0x17ca0:0x17ca0+134]
    embedded = gzip.decompress(compressed)
    script = inputs['decrypted-0.bin'].decode('ascii')
    anchors = {'format_root':'mkfs.ext2 -b 4096', 'mount_root':'mount -t ext2',
               'read_update_filename':'_UPDATE_FN_=', 'check_package_exists':'if [ ! -f /contents/',
               'opaque_fwpup_tar_pipeline':'fwpup -f', 'sync':'sync', 'unmount':'umount /mnt'}
    anchor_lines = {}
    for key, prefix in anchors.items():
        matching = [i+1 for i,line in enumerate(script.splitlines()) if line.startswith(prefix)]
        if len(matching) != 1:
            raise ValueError('Stock script anchor mismatch: '+key)
        anchor_lines[key] = matching[0]
    output.mkdir(parents=True,exist_ok=False)
    (output/'initrd-inventory.json').write_text(json.dumps(init_entries,indent=2)+'\n',encoding='utf-8')
    (output/'rootfs-inventory.json').write_text(json.dumps(root_entries,indent=2)+'\n',encoding='utf-8')
    root_paths = {member['path'] for member in root_entries}
    facts = {'evidence_kind':'static_file_content_not_device_state','physical_device_access':False,'sony_code_executed':False,
             'input_sha256':{name:sha(data) for name,data in inputs.items()},
             'rootfs_members':len(root_entries),'initrd_entries':len(init_entries),
             'initrd_geometry':{'block_size':initrd.block_size,'filesystem_blocks':initrd.block_count,'image_bytes':len(initrd.data)},
             'initrd_linuxrc_text':init_data['/linuxrc'].decode('ascii'),
             'rootfs_keyword_hits':searches,'nvp_symbol_importers':importers,'elf_inputs_not_parsed':unparsable,
             'named_fwpup_in_rootfs':[path for path in root_paths if path.rsplit('/',1)[-1] == 'fwpup'],
             'named_fwpup_in_initrd':[row['path'] for row in init_entries if row['path'].rsplit('/',1)[-1] == 'fwpup'],
             'raw_kernel_literal_hits':hits(inputs['decrypted-1.bin']),
             'kernel_embedded_initramfs':{'offset':'0x17ca0','compressed_bytes':134,'expanded_bytes':len(embedded),
                                         'compressed_sha256':sha(compressed),'expanded_sha256':sha(embedded),'entries':newc_entries(embedded)},
             'nvp_flag_mapping':{'driver_sha256':sha(module_bytes),'minor_index':9,'zone':node[0],'node':node[1],
                                 'logical_zone_tag':'0x70','logical_zone_bytes':2048,
                                 'node_table':'MSCZone','table_section':target_section,'table_section_offset':target_offset,
                                 'relocation_kind':'R_ARM_ABS32','logical_field_offset':field[0],'field_bytes':field[1],
                                 'physical_nand_address_unknown':True},
             'kernel_code_bound_to_raw_input':kernel_bindings,
             'stock_script':{'sha256':PINS['decrypted-0.bin'],'anchors_1_based':anchor_lines,
                             'explicit_fup_clear_commands_found':False,'fwpup_internal_write_order':'unknown'},
             'startup_symlinks':[row for row in root_entries if 'target'in row and row['path'].startswith(('./etc/rcS.d/','./etc/rc2.d/'))],
             'powerfail_script_in_rootfs':'./etc/init.d/powerfail'in root_paths,
             'powerfail_references':[line for line in root_data['./etc/inittab'].decode('ascii').splitlines() if 'powerfail'in line],
             'limitations':['Negative searches cover these pinned inputs only, not all device storage',
                            'No resident boot selector or fwpup executable recovered',
                            'NVP metadata recovery is not full-system rollback',
                            'Actual geometry, flags, NAND state, power-loss behavior and rescue access not measured'],
             'static_exports':[]}
    if objdump:
        if objdump.name.lower() not in ('llvm-objdump','llvm-objdump.exe'):
            raise ValueError('Expected llvm-objdump as static reader')
        commands = [('apl-flag',work/'rootfs/usr/local/bin/apl_conn', ['-d','--start-address=0x8d88','--stop-address=0x8e88']),
                    ('apl-flag-caller',work/'rootfs/usr/local/bin/apl_conn',['-d','--start-address=0x8a80','--stop-address=0x8ae0']),
                    ('gemini-flag',work/'rootfs/usr/local/bin/gemini-ICX1101',['-d','--start-address=0x1a4ffc','--stop-address=0x1a50a8']),
                    ('nvp-module',work/'rootfs/lib/modules/icx1087_nvp.ko',['-dr']),
                    ('nand-i-module',work/'rootfs/lib/modules/icx1087-nand-i.ko',['-dr']),
                    ('nand-p-module',work/'rootfs/lib/modules/icx1087-nand-p.ko',['-dr']),
                    ('kernel-write',work/'kernel-recovered.elf',['-d','--start-address=0xc01459f4','--stop-address=0xc0145eb4']),
                    ('kernel-erase',work/'kernel-recovered.elf',['-d','--start-address=0xc0145eb4','--stop-address=0xc01462ac'])]
        for name, path, flags in commands:
            if 'rootfs'in path.parts:
                relative='./'+path.relative_to(work/'rootfs').as_posix()
                if path.read_bytes()!=root_data[relative]:
                    raise ValueError('Extracted input differs from original tar: '+relative)
            result=subprocess.run([str(objdump),*flags,str(path)],capture_output=True,text=True,encoding='utf-8',check=True,timeout=30)
            data=result.stdout.encode('utf-8');filename=name+'.txt';(output/filename).write_bytes(data)
            facts['static_exports'].append({'file':filename,'sha256':sha(data),'input_sha256':sha(path.read_bytes())})
    if sevenzip:
        package=work.parent/'firmware/original/NW-A910_V1_03.exe'
        if sha(package.read_bytes())!='6f745a8ff3c90b7abcd5ea185e6f51e4598492f6de5bdb3256599c6f43e39701':
            raise ValueError('Outer official archive hash mismatch')
        if sevenzip.name.lower() not in ('7z','7z.exe'):
            raise ValueError('Expected 7z as archive reader')
        result=subprocess.run([str(sevenzip),'l',str(package)],capture_output=True,text=True,encoding='utf-8',check=True,timeout=30)
        data=result.stdout.encode('utf-8');(output/'outer-package-list.txt').write_bytes(data)
        facts['outer_archive']={'sha256':sha(package.read_bytes()),'listing_sha256':sha(data),'listing_only':True}
    (output/'facts.json').write_text(json.dumps(facts,indent=2)+'\n',encoding='utf-8')
    return facts


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--work',required=True,type=Path)
    parser.add_argument('--output',required=True,type=Path)
    parser.add_argument('--objdump',type=Path)
    parser.add_argument('--sevenzip',type=Path)
    args=parser.parse_args()
    facts=collect(args.work.resolve(strict=True),args.output.resolve(),args.objdump.resolve(strict=True)if args.objdump else None,
                  args.sevenzip.resolve(strict=True)if args.sevenzip else None)
    print(json.dumps({'rootfs_members':facts['rootfs_members'],'initrd_entries':facts['initrd_entries'],
                      'flag_mapping':facts['nvp_flag_mapping'],'fwpup_recovered':False,'output':str(args.output)},indent=2))
