"""Host-only fault reproduction for the stock install script's pipeline.

Never loads or runs Sony code. A synthetic empty gzip/tar stream is produced by
host Python and consumed by host tar in list mode (no extraction or writes).
This verifies POSIX pipeline semantics, not the resident shell or flash behavior.
"""
import base64
import gzip
import io
from pathlib import Path
import shlex
import shutil
import subprocess
import sys
import tarfile
import unittest


class FirmwarePipelineFaults(unittest.TestCase):
    def pipeline(self, producer_status, valid_archive):
        shell = shutil.which('sh')
        if sys.platform == 'win32':
            shell = str(Path('C:/Program Files/Git/bin/bash.exe'))
        if not shell or not Path(shell).is_file():
            self.skipTest('Host POSIX shell is unavailable')
        raw = io.BytesIO()
        with tarfile.open(fileobj=raw, mode='w'):
            pass
        payload = gzip.compress(raw.getvalue()) if valid_archive else b'not gzip'
        code = (
            'import base64,sys;sys.stdout.buffer.write(base64.b64decode('
            + repr(base64.b64encode(payload).decode('ascii'))
            + '));sys.stdout.buffer.flush();sys.exit('
            + str(producer_status) + ')'
        )
        # No firmware script, device path, extraction, flash tool or NVP access.
        command = (shlex.quote(sys.executable.replace('\\', '/')) + ' -c '
                   + shlex.quote(code) + ' | tar -tzf - >/dev/null')
        return subprocess.run(
            [shell, '-c', command], capture_output=True, timeout=15
        ).returncode

    def test_successful_producer_and_archive(self):
        self.assertEqual(self.pipeline(0, True), 0)

    def test_failed_producer_can_be_masked_by_valid_archive(self):
        self.assertEqual(self.pipeline(7, True), 0)

    def test_invalid_archive_is_reported(self):
        self.assertNotEqual(self.pipeline(7, False), 0)


if __name__ == '__main__':
    unittest.main()
