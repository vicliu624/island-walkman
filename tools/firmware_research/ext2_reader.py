"""Read ext2 bytes without mounting, extracting or executing image contents."""
import struct


class Ext2Image:
    def __init__(self, data):
        self.data = data
        if len(data) < 2048 or self.u16(1080) != 0xEF53:
            raise ValueError('Not an ext2 image')
        log_block_size = self.u32(1048)
        if log_block_size > 2:
            raise ValueError('Unsupported ext2 block size')
        self.block_size = 1024 << log_block_size
        self.block_count = self.u32(1028)
        self.inode_count = self.u32(1024)
        self.inodes_per_group = self.u32(1064)
        self.inode_size = self.u16(1112) if self.u32(1100) else 128
        self.filetype_entries = bool(self.u32(1120) & 2)
        if self.u32(1120) & ~2:
            raise ValueError('Unsupported ext2 incompatible features')
        if (self.block_size not in (1024, 2048, 4096) or not self.inodes_per_group
                or not self.inode_count or not self.block_count or self.inode_size < 128
                or self.inode_size > self.block_size
                or self.block_count * self.block_size > len(data)):
            raise ValueError('Invalid or truncated ext2 geometry')
        self.gdt = self.block_size * (2 if self.block_size == 1024 else 1)

    def read(self, offset, size):
        if offset < 0 or size < 0 or offset + size > len(self.data):
            raise ValueError('Read outside image')
        return self.data[offset:offset + size]

    def u16(self, offset):
        return struct.unpack('<H', self.read(offset, 2))[0]

    def u32(self, offset):
        return struct.unpack('<I', self.read(offset, 4))[0]

    def block(self, number):
        if not 0 < number < self.block_count:
            raise ValueError('Invalid block number')
        return self.read(number * self.block_size, self.block_size)

    def inode(self, number):
        if not 1 <= number <= self.inode_count:
            raise ValueError('Invalid inode number')
        group, slot = divmod(number - 1, self.inodes_per_group)
        table = self.u32(self.gdt + group * 32 + 8)
        if not 0 < table < self.block_count:
            raise ValueError('Invalid inode table')
        return self.read(table * self.block_size + slot * self.inode_size, self.inode_size)

    def contents(self, number):
        inode = self.inode(number)
        size = struct.unpack_from('<I', inode, 4)[0]
        if size > len(self.data):
            raise ValueError('Oversized inode')
        mode = struct.unpack_from('<H', inode)[0] & 0xF000
        if mode == 0xA000 and size <= 60 and struct.unpack_from('<I', inode, 28)[0] == 0:
            return inode[40:40 + size]
        pointers = list(struct.unpack_from('<15I', inode, 40))
        count = (size + self.block_size - 1) // self.block_size
        blocks = pointers[:12]
        per_block = self.block_size // 4

        def indirect(pointer):
            raw = self.block(pointer) if pointer else bytes(self.block_size)
            return list(struct.unpack('<' + 'I' * per_block, raw))

        if count > 12:
            blocks += indirect(pointers[12])
        if count > 12 + per_block:
            if count > 12 + per_block + per_block * per_block:
                raise ValueError('Triple-indirect inode is outside this bounded reader')
            for pointer in indirect(pointers[13]):
                if len(blocks) >= count:
                    break
                blocks += indirect(pointer)
        data = b''.join(self.block(block) if block else bytes(self.block_size) for block in blocks[:count])
        return data[:size]

    def entries(self):
        output, visited = [], set()

        def walk(number, path, depth):
            if depth > 32 or len(output) > 4096:
                raise ValueError('Directory traversal limit')
            inode = self.inode(number)
            mode = struct.unpack_from('<H', inode)[0]
            kind = mode & 0xF000
            row = {'path':path, 'inode':number, 'mode':oct(mode), 'kind':kind,
                   'size':struct.unpack_from('<I', inode, 4)[0]}
            if kind == 0xA000:
                row['target'] = self.contents(number).decode('utf-8')
            output.append(row)
            if kind != 0x4000:
                return
            if number in visited:
                raise ValueError('Directory cycle')
            visited.add(number)
            data, offset = self.contents(number), 0
            while offset < len(data):
                if offset + 8 > len(data):
                    raise ValueError('Truncated directory entry')
                ino, length = struct.unpack('<IH', data[offset:offset + 6])
                if (length < 8 or offset + length > len(data) or length % 4
                        or offset % self.block_size + length > self.block_size):
                    raise ValueError('Invalid directory entry')
                name_length = data[offset + 6] if self.filetype_entries else struct.unpack_from('<H',data,offset + 6)[0]
                if name_length > length - 8:
                    raise ValueError('Invalid directory name length')
                name = data[offset + 8:offset + 8 + name_length].decode('utf-8')
                if ino and name not in ('.', '..'):
                    if not name or '/' in name or '\0' in name:
                        raise ValueError('Invalid directory name')
                    walk(ino, path.rstrip('/') + '/' + name, depth + 1)
                offset += length
        walk(2, '/', 0)
        return output
