"""Complete the public Sony firmware catalog with stable ordering and coverage checks.

Metadata GETs only; no firmware execution, device access or image mounting.
"""
import argparse
from concurrent.futures import ThreadPoolExecutor
import hashlib
import json
import math
from pathlib import Path
import re
import urllib.parse
from search_resident_catalogs import fetch

QUERY = '(title:sony OR creator:sony) AND (title:firmware OR title:recovery OR title:backup)'
FIELDS = ['identifier', 'title', 'creator', 'mediatype', 'description', 'subject']
PATTERNS = {
    'exact_family': r'(?i)\b(?:NW[\s_-]*A[\s_-]*91[0689]|A91[0689]|ICX[\s_-]*1101)\b',
    'resident_terms': r'(?i)linuxrc_fwup|fwpup|fUKt|fURt|FUINITRD|FUKERNEL|wrldr',
    'walkman': r'(?i)walkman|ウォークマン|nw[\s_-]*(?:a|s|e|z)',
    'generic_capture': r'(?i)\b(?:nand|dump|factory|flash|bootloader|backup|firmware)\b',
}


def url(page):
    parameters = {'q': QUERY, 'output': 'json', 'rows': 200, 'page': page,
                  'sort[]': 'identifier asc', 'fl[]': FIELDS}
    return 'https://archive.org/advancedsearch.php?' + urllib.parse.urlencode(parameters, doseq=True)


def complete(prior, output, cached=False):
    baseline = json.loads(prior.read_bytes())['response']
    if cached:
        requests = json.loads((output/'requests.json').read_text(encoding='utf-8'))
        first = requests[0]
        for page, request in enumerate(requests, 1):
            payload = (output/(request['name']+'.response')).read_bytes()
            if request['url'] != url(page) or hashlib.sha256(payload).hexdigest() != request['sha256']:
                raise ValueError('Cached response provenance mismatch')
    else:
        output.mkdir(parents=True, exist_ok=False)
        first = fetch(('page-1', url(1)), output)
        requests = [first]
    if 'error' in first:
        raise ValueError(first['error'])
    total = first['num_found']
    if total > 3000:
        raise ValueError('Unexpected catalog growth; revise explicit bound')
    tasks = [('page-'+str(page), url(page)) for page in range(2, math.ceil(total/200)+1)]
    if not cached:
        with ThreadPoolExecutor(max_workers=3) as pool:
            requests.extend(pool.map(lambda task: fetch(task, output), tasks))
        (output/'requests.json').write_text(json.dumps(requests, indent=2)+'\n', encoding='utf-8')
    documents = []
    for request in requests:
        if 'error' in request:
            raise ValueError(request['error'])
        body = json.loads((output/(request['name']+'.response')).read_bytes())['response']
        if body['numFound'] != total:
            raise ValueError('Catalog changed between pages; coverage cannot be claimed')
        documents.extend(body['docs'])
    identifiers = [doc['identifier'] for doc in documents]
    # The archive's identifier sort is case-insensitive; identifiers retain case.
    if len(identifiers) != total or len(set(identifiers)) != total or identifiers != sorted(identifiers, key=str.casefold):
        raise ValueError('Incomplete, duplicate or unstable pagination')
    prior_ids = {doc['identifier'] for doc in baseline['docs']}
    missing = sorted(prior_ids-set(identifiers))
    if missing:
        raise ValueError('Prior records absent from current catalog: ' + str(missing))
    rows = []
    for doc in documents:
        text = json.dumps(doc, ensure_ascii=False)
        hits = {name: sorted(set(re.findall(pattern, text))) for name, pattern in PATTERNS.items()}
        rows.append(dict(identifier=doc['identifier'], title=doc.get('title'),
                         previously_returned=doc['identifier'] in prior_ids,
                         signals={name: terms for name, terms in hits.items() if terms}))
    coverage = dict(query=QUERY, sort='identifier asc', total=total, unique=len(set(identifiers)),
                    prior_response_sha256=hashlib.sha256(prior.read_bytes()).hexdigest(),
                    page_counts=[r['returned'] for r in requests], prior_total=baseline['numFound'],
                    prior_returned=len(prior_ids), previously_unreviewed=len(set(identifiers)-prior_ids),
                    prior_missing=missing,
                    limits='Complete catalog metadata coverage for this query snapshot; not complete file-content or global archive coverage.')
    for name, value in [('coverage.json', coverage), ('records.json', documents), ('signals.json', rows)]:
        (output/name).write_text(json.dumps(value, indent=2, ensure_ascii=True)+'\n', encoding='utf-8')
    print(json.dumps(coverage, indent=2))
    print(json.dumps([row for row in rows if any(key in row['signals'] for key in ['exact_family','resident_terms','walkman'])], indent=2))


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--prior', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--cached', action='store_true', help='Verify and analyze previously saved page responses without HTTP')
    args = parser.parse_args()
    complete(args.prior, args.output, args.cached)
