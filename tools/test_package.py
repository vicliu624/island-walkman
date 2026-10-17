"""Regression checks for the archive format used by release packages."""
import os
from pathlib import Path
import tempfile
import unittest
import zipfile

from package import write_zip


class ZipPackagingTest(unittest.TestCase):
    def test_registry_license_with_old_timestamp_is_preserved(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            stage = root/'application'
            license = stage/'third-party-licenses'/'dependency'/'LICENSE'
            license.parent.mkdir(parents=True)
            license.write_text('Synthetic license notice\n')
            os.utime(license, (1, 1))
            archive = root/'application.zip'
            write_zip(stage, archive)
            with zipfile.ZipFile(archive) as zipped:
                name = 'application/third-party-licenses/dependency/LICENSE'
                self.assertEqual(zipped.read(name), license.read_bytes())
                self.assertEqual(zipped.getinfo(name).date_time[:3], (1980, 1, 1))


if __name__ == '__main__':
    unittest.main()
