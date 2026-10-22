"""Small read-only ELF32/ARM metadata reader; no loader or execution API."""
import struct


class Elf32:
    def __init__(self, data):
        self.data = data
        if len(data) < 52 or data[:6] != b'\x7fELF\x01\x01':
            raise ValueError('Expected ELF32 little-endian')
        self.kind, self.machine = struct.unpack_from('<HH', data, 16)
        if self.machine != 40:
            raise ValueError('Expected ARM')
        offset = struct.unpack_from('<I', data, 32)[0]
        stride, count, strings = struct.unpack_from('<HHH', data, 46)
        if stride < 40 or not 0 < count < 4096 or strings >= count:
            raise ValueError('Invalid section table')
        self.sections = [struct.unpack('<10I', self.read(offset + i * stride, 40)) for i in range(count)]
        names = self.section_bytes(strings)
        self.names = [self.cstring(names, section[0]) for section in self.sections]
        self.symbols = []
        self.tables = {}
        for index, section in enumerate(self.sections):
            if section[1] not in (2, 11):
                continue
            if section[5] % 16 or section[9] != 16:
                raise ValueError('Unsupported symbol table')
            strings = self.section_bytes(section[6])
            symbols = []
            for at in range(0, section[5], 16):
                name, value, size, info, other, ndx = struct.unpack('<IIIBBH', self.read(section[4] + at, 16))
                symbol = {'name':self.cstring(strings,name),'value':value,'size':size,'info':info,'section':ndx}
                symbols.append(symbol)
                self.symbols.append(symbol)
            self.tables[index] = symbols

    def read(self, offset, size):
        if offset < 0 or size < 0 or offset + size > len(self.data):
            raise ValueError('Read outside ELF')
        return self.data[offset:offset + size]

    @staticmethod
    def cstring(data, offset):
        if not 0 <= offset < len(data):
            raise ValueError('Invalid string offset')
        return data[offset:data.index(0,offset)].decode('ascii')

    def section_bytes(self, index):
        section = self.sections[index]
        if section[1] == 8:
            raise ValueError('No file content for BSS')
        return self.read(section[4], section[5])

    def symbol(self, name):
        matches = [symbol for symbol in self.symbols if symbol['name'] == name and 0 < symbol['section'] < len(self.sections)]
        if len(matches) != 1:
            raise ValueError('Expected one defined symbol: ' + name)
        return matches[0]

    def symbol_bytes(self, name):
        symbol = self.symbol(name)
        section = self.sections[symbol['section']]
        offset = symbol['value'] - (section[3] if self.kind != 1 else 0)
        if offset < 0 or offset + symbol['size'] > section[5]:
            raise ValueError('Invalid symbol extent')
        return self.read(section[4] + offset, symbol['size'])

    def absolute_relocation(self, target_section, target_offset):
        for section in self.sections:
            if section[1] != 9 or self.names[section[7]] != target_section:
                continue
            for at in range(section[4],section[4] + section[5],8):
                offset, info = struct.unpack('<II',self.read(at,8))
                if offset != target_offset:
                    continue
                symbol = self.tables[section[6]][info >> 8]
                if info & 255 != 2 or not 0 < symbol['section'] < len(self.sections):
                    raise ValueError('Expected file-backed R_ARM_ABS32')
                raw = self.sections[section[7]]
                addend = struct.unpack('<I',self.read(raw[4] + offset,4))[0]
                return self.names[symbol['section']], symbol['value'] + addend
        raise ValueError('Relocation missing')

    def read_va(self, address, size):
        for section in self.sections:
            if section[1] == 8:
                continue
            if section[3] <= address and address + size <= section[3] + section[5]:
                return self.read(section[4] + address - section[3], size)
        raise ValueError('VA not file backed')
