"""Read public archive metadata only. No device API or target execution.

Save bounded HTTP responses and request provenance to a new local directory.
Catalog matches are candidates, never compatibility/authenticity evidence.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
from pathlib import Path
import urllib.parse
import urllib.request

QUERIES = [
    ('ia-exact-models', '"NW-A916" OR "NW-A918" OR "NW-A919" OR "NW-A910"'),
    ('ia-compact-models', '"NWA916" OR "NWA918" OR "NWA919" OR "NW_A91X"'),
    ('ia-platform', '"ICX1101" OR "FUINITRD" OR "FUKERNEL" OR "linuxrc_fwup"'),
    ('ia-fwpup', '"fwpup"'),
    ('ia-walkman-software', '(title:walkman OR description:walkman) AND mediatype:software'),
    ('ia-sony-firmware', '(title:sony OR creator:sony) AND (title:firmware OR title:recovery OR title:backup)'),
]


def requests():
    for name, query in QUERIES:
        parameters = {'q': query, 'output': 'json', 'rows': 200, 'page': 1,
                      'fl[]': ['identifier', 'title', 'mediatype', 'description']}
        yield name, 'https://archive.org/advancedsearch.php?' + urllib.parse.urlencode(parameters, doseq=True)
    for name, pattern in [
        ('cdx-official-distribution', 'walkman.update.sony.net/fw/pc/A910/*'),
        ('cdx-old-distribution', 'www.sony.jp/support/walkman/download/*a916*'),
        ('cdx-source-distribution', 'prodgpl.blob.core.windows.net/download/Audio/NW-A916/*'),
    ]:
        parameters = {'url': pattern, 'output': 'json', 'filter': 'statuscode:200',
                      'collapse': 'urlkey', 'fl': 'timestamp,original,mimetype,statuscode,digest,length', 'limit': 1000}
        yield name, 'https://web.archive.org/cdx/search/cdx?' + urllib.parse.urlencode(parameters)


def fetch(task, output):
    name, url = task
    row = dict(name=name, url=url, method='GET')
    try:
        request = urllib.request.Request(url, headers={'User-Agent': 'IslandWalkman-OfflineResearch/1.0'})
        with urllib.request.urlopen(request, timeout=15) as response:
            data = response.read(4*1024*1024+1)
            if len(data) > 4*1024*1024:
                raise ValueError('Metadata response exceeds 4 MiB bound')
            row.update(status=response.status, final_url=response.url, bytes=len(data),
                       sha256=hashlib.sha256(data).hexdigest())
            (output/(name+'.response')).write_bytes(data)
            body = json.loads(data)
            if isinstance(body, dict) and 'response' in body:
                row['num_found'] = body['response']['numFound']
                row['returned'] = len(body['response']['docs'])
            else:
                row['returned'] = max(0, len(body)-1)
    except Exception as error:
        row['error'] = type(error).__name__ + ': ' + str(error)
    return row


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=False)
    with ThreadPoolExecutor(max_workers=4) as pool:
        records = list(pool.map(lambda task: fetch(task, args.output), requests()))
    (args.output/'requests.json').write_text(json.dumps(records, indent=2)+'\n', encoding='utf-8')
    for row in records:
        print(json.dumps({key: row[key] for key in ('name', 'status', 'num_found', 'returned', 'error') if key in row}))
