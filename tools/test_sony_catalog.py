"""Synthetic offline coverage checks; never call the public archive or a device."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest
from unittest.mock import patch
import sys

sys.path.insert(0, str(Path(__file__).parent/'firmware_research'))
from complete_sony_catalog import complete, url


class SonyCatalogCoverageTests(unittest.TestCase):
    def fixture(self, root, pages, prior_ids, totals=None):
        output = root/'pages'
        output.mkdir()
        total = sum(len(page) for page in pages)
        totals = totals or [total]*len(pages)
        requests = []
        for index, identifiers in enumerate(pages, 1):
            payload = json.dumps({'response': {'numFound': totals[index-1], 'docs': [
                {'identifier': ident, 'title': 'Synthetic item'} for ident in identifiers]}}).encode()
            name = 'page-'+str(index)
            (output/(name+'.response')).write_bytes(payload)
            requests.append(dict(name=name, url=url(index), sha256=hashlib.sha256(payload).hexdigest(),
                                 num_found=totals[index-1], returned=len(identifiers)))
        (output/'requests.json').write_text(json.dumps(requests), encoding='utf-8')
        prior = root/'prior.json'
        prior.write_text(json.dumps({'response': {'numFound': total, 'docs': [
            {'identifier': ident} for ident in prior_ids]}}), encoding='utf-8')
        return prior, output

    def test_case_preserving_archive_order_and_remaining_count(self):
        with tempfile.TemporaryDirectory() as temp:
            prior, output = self.fixture(Path(temp), [['a', 'B', 'c']], ['B'])
            with patch('builtins.print'):
                complete(prior, output, cached=True)
            coverage = json.loads((output/'coverage.json').read_text())
            self.assertEqual(coverage['previously_unreviewed'], 2)
            self.assertEqual(coverage['unique'], 3)
            self.assertEqual(coverage['prior_missing'], [])

    def test_duplicates_cannot_be_reported_as_complete(self):
        with tempfile.TemporaryDirectory() as temp:
            prior, output = self.fixture(Path(temp), [['a', 'a']], ['a'])
            with self.assertRaisesRegex(ValueError, 'Incomplete, duplicate'):
                complete(prior, output, cached=True)
            self.assertFalse((output/'coverage.json').exists())

    def test_missing_prior_record_blocks_completion_claim(self):
        with tempfile.TemporaryDirectory() as temp:
            prior, output = self.fixture(Path(temp), [['b']], ['a'])
            with self.assertRaisesRegex(ValueError, 'Prior records absent'):
                complete(prior, output, cached=True)

    def test_changed_total_between_pages_blocks_completion_claim(self):
        with tempfile.TemporaryDirectory() as temp:
            prior, output = self.fixture(Path(temp), [['a'], ['b']], ['a'], [2, 3])
            with self.assertRaisesRegex(ValueError, 'Catalog changed'):
                complete(prior, output, cached=True)

    def test_modified_cached_page_rejected_before_analysis(self):
        with tempfile.TemporaryDirectory() as temp:
            prior, output = self.fixture(Path(temp), [['a']], ['a'])
            (output/'page-1.response').write_bytes(b'{}')
            with self.assertRaisesRegex(ValueError, 'provenance mismatch'):
                complete(prior, output, cached=True)


if __name__ == '__main__':
    unittest.main()
