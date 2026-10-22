"""Regression checks for the offline archive reader, without firmware/devices."""
import hashlib
import io
from pathlib import Path
import tarfile
import tempfile
import unittest
from firmware_research.compare_a910_rootfs import inventory


class RootfsInventoryTests(unittest.TestCase):
    def read_fixture(self, members):
        buffer = io.BytesIO()
        with tarfile.open(fileobj=buffer, mode='w:gz') as archive:
            for info, payload in members:
                archive.addfile(info, io.BytesIO(payload) if payload is not None else None)
        raw = buffer.getvalue()
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'input.gz'
            path.write_bytes(raw)
            return inventory(path, hashlib.sha256(raw).hexdigest())

    def test_pin_rejects_unexpected_input_before_parsing(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory)/'input.gz'
            path.write_bytes(b'not an archive')
            with self.assertRaisesRegex(ValueError, 'Unexpected input hash'):
                inventory(path, '0'*64)

    def test_symlink_is_recorded_without_following_host_target(self):
        info = tarfile.TarInfo('linuxrc_fwup')
        info.type = tarfile.SYMTYPE
        info.linkname = '/definitely-not-a-host-file/fwpup'
        entries, payloads, matches = self.read_fixture([(info, None)])
        self.assertEqual(entries[0]['target'], info.linkname)
        self.assertEqual(payloads, {})
        self.assertEqual(matches, [])

    def test_parent_path_is_metadata_not_extracted(self):
        info = tarfile.TarInfo('../fwpup')
        info.size = 5
        entries, payloads, matches = self.read_fixture([(info, b'fwpup')])
        self.assertEqual(entries[0]['path'], '../fwpup')
        self.assertEqual(payloads['../fwpup'], b'fwpup')
        self.assertEqual(matches[0]['hits']['fwpup'], [0])

    def test_duplicate_paths_are_not_silently_collapsed(self):
        one = tarfile.TarInfo('same')
        two = tarfile.TarInfo('same')
        with self.assertRaisesRegex(ValueError, 'Duplicate member paths'):
            self.read_fixture([(one, b''), (two, b'')])

    def test_binary_literal_offsets_are_preserved(self):
        info = tarfile.TarInfo('binary')
        payload = b'\x00fwpup\xfffwpup\x00/dev/icx1087_nvp/9'
        info.size = len(payload)
        entries, _, matches = self.read_fixture([(info, payload)])
        self.assertEqual(matches[0]['hits']['fwpup'], [1, 7])
        self.assertEqual(entries[0]['sha256'], hashlib.sha256(payload).hexdigest())


if __name__ == '__main__':
    unittest.main()
