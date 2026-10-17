"""Exercise a packaged executable without accessing a real player or personal music."""
import argparse
import hashlib
import json
from pathlib import Path
import re
import socket
import subprocess
import tempfile
import time
import urllib.request


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('--executable', required=True, type=Path)
    args = parser.parse_args()
    executable = args.executable.resolve(strict=True)
    with tempfile.TemporaryDirectory(prefix='island-walkman-smoke-') as folder:
        root = Path(folder)
        music = root/'Generated test audio.mp3'
        # MPEG-1 Layer III, 128 kbit/s, 44.1 kHz. Synthetic silence, no copyrighted fixture.
        music.write_bytes((bytes.fromhex('fffb9000')+bytes(413))*100)
        before = hashlib.sha256(music.read_bytes()).hexdigest()
        data = root/'data'
        subprocess.run([str(executable), '--no-open', '--data', str(data), '--import', str(music), '--prepare'], check=True, timeout=60)
        preparations = list((data/'preparations').glob('*/manifest.json'))
        assert len(preparations) == 1, 'No local preparation manifest'
        assert hashlib.sha256(music.read_bytes()).hexdigest() == before, 'Source changed'
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0))
            port = sock.getsockname()[1]
        process = subprocess.Popen([str(executable), '--no-open', '--data', str(data), '--port', str(port)], stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
        try:
            base = f'http://127.0.0.1:{port}'
            for attempt in range(100):
                if process.poll() is not None:
                    raise RuntimeError(process.communicate()[0].decode(errors='replace'))
                try:
                    with urllib.request.urlopen(base+'/api/state', timeout=1) as response:
                        state = json.load(response)
                    break
                except OSError:
                    time.sleep(.1)
            else:
                raise RuntimeError('Local server did not start')
            assert state['device_write_enabled'] is False
            assert len(state['state']['tracks']) == 1
            with urllib.request.urlopen(base+'/', timeout=5) as response:
                html = response.read().decode()
            assert 'lang="en"' in html
            assets = re.findall(r'(?:src|href)="(/assets/[^\"]+)"', html)
            assert assets, 'Packaged UI assets missing'
            for asset in assets:
                with urllib.request.urlopen(base+asset, timeout=5) as response:
                    assert len(response.read()) > 0
            print('Package smoke test passed: source intact, local preparation, API and UI assets.')
        finally:
            process.terminate()
            try:
                process.communicate(timeout=10)
            except subprocess.TimeoutExpired:
                process.kill()
                process.communicate(timeout=10)


if __name__ == '__main__':
    main()
