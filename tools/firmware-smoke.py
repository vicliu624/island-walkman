"""Exercise native USB inventory and synthetic UPG validation without player writes.

The 96-byte fixture was independently encrypted with .NET DES ECB. It contains
four eight-byte zero components and is not installable firmware. No Sony binary,
ROM, driver, elevated access or Python crypto package is needed.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import tempfile

FIXTURE = bytes.fromhex(
    '793dbb93bfdbcc60d08e530f1b297833'
    '0955c89caf08e846c997f5dbb90ee9c366036a531fdea94fcc8ca2b5af2b40557'
    'a071007c97010b8a24778542b8e1592ada'
    'fad6697223c35adafad6697223c35adafad6697223c35adafad6697223c35'
)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--executable', required=True, type=Path)
    args = parser.parse_args()
    executable = str(args.executable.resolve(strict=True))
    with tempfile.TemporaryDirectory(prefix='island-firmware-readonly-') as folder:
        root = Path(folder)
        data = root / 'must-not-create-catalog'
        package = root / 'synthetic.upg'
        package.write_bytes(FIXTURE)

        def invoke(*arguments):
            return subprocess.run([executable, '--data', str(data), *arguments],
                                  capture_output=True, text=True, encoding='utf-8', timeout=45)

        result = invoke('--inspect-firmware', str(package))
        assert result.returncode == 0, result.stderr
        report = json.loads(result.stdout)
        assert len(report['components']) == 4
        assert report['sha256'] == hashlib.sha256(FIXTURE).hexdigest()
        assert all(c['sha256'] == hashlib.sha256(bytes(8)).hexdigest() for c in report['components'])
        for flag in ['authenticity_verified', 'device_compatibility_verified',
                     'firmware_write_enabled', 'stock_103_exact_match']:
            assert report[flag] is False
        assert package.read_bytes() == FIXTURE
        damaged = bytearray(FIXTURE)
        damaged[-1] ^= 1
        package.write_bytes(damaged)
        assert invoke('--inspect-firmware', str(package)).returncode != 0
        assert package.read_bytes() == damaged
        assert invoke('--inspect-firmware', str(root / 'missing.upg')).returncode != 0
        assert invoke('--inspect-firmware').returncode != 0
        result = invoke('--firmware-probe')
        assert result.returncode == 0, result.stderr
        report = json.loads(result.stdout)
        assert report['mode'] == 'usb_inventory_only'
        assert report['firmware_write_enabled'] is False
        assert report['vendor_queries_enabled'] is False
        assert report['installed_firmware'] is None
        assert report['battery_percent'] is None
        assert all(d['model_verified'] is False for d in report['candidates'])
        # Music-transfer authorization must never enable a firmware operation.
        result = invoke('--allow-device-write', '--firmware-probe')
        assert result.returncode == 0, result.stderr
        assert json.loads(result.stdout)['firmware_write_enabled'] is False
        assert not data.exists(), 'Firmware CLI must bypass catalog loading and recovery'
    print('Firmware smoke passed: native USB inventory, independent synthetic UPG, '
          'corruption rejection, unchanged inputs and no catalog/recovery startup.')


if __name__ == '__main__':
    main()
