"""License provenance/byte-integrity regressions; no network or Cargo calls."""
import hashlib
import json
from pathlib import Path
import tempfile
import unittest

from scripts.generate_notices import (
    MAX_TEXT_BYTES, cached_license, shipped_license_files,
)


class NoticeTests(unittest.TestCase):
    def test_shipped_nested_licenses_and_reviewed_authors_are_included(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            (root / 'LICENSES').mkdir()
            nested = root / 'LICENSES' / 'MIT.txt'
            nested.write_text('Isolated test fixture')
            authors = root / 'AUTHORS'
            authors.write_text('Isolated test fixture')
            package = {'name': 'r-efi', 'version': '6.0.0', 'manifest_path': str(root / 'Cargo.toml')}
            self.assertEqual(shipped_license_files(package), {nested, authors})
            package['name'] = 'unreviewed-fixture'
            self.assertEqual(shipped_license_files(package), {nested})

    def test_cached_bytes_match_provenance_without_newline_normalization(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'LICENSE'
            data = 'Fixture grant\r\n© Fixture\r\n'.encode()
            path.write_bytes(data)
            self.provenance(path, data)
            self.assertEqual(cached_license(path).encode(), data)

    def test_tampered_text_or_missing_provenance_cannot_generate_notices(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'LICENSE'
            path.write_bytes(b'Fixture grant')
            self.provenance(path, b'Different grant')
            with self.assertRaisesRegex(ValueError, 'checksum mismatch'):
                cached_license(path)
            path.with_name('LICENSE.source.json').unlink()
            with self.assertRaises(FileNotFoundError):
                cached_license(path)

    def test_oversized_text_and_non_https_provenance_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            path = Path(temporary) / 'LICENSE'
            path.write_bytes(b'x' * (MAX_TEXT_BYTES + 1))
            with self.assertRaisesRegex(ValueError, 'size limit'):
                cached_license(path)
            path.write_bytes(b'Fixture grant')
            self.provenance(path, b'Fixture grant', 'http://example.invalid/LICENSE')
            with self.assertRaisesRegex(ValueError, 'HTTPS source'):
                cached_license(path)

    @staticmethod
    def provenance(path, data, url='https://example.invalid/pinned-commit/LICENSE'):
        path.with_name(path.name + '.source.json').write_text(json.dumps({
            'url': url, 'sha256': hashlib.sha256(data).hexdigest(),
        }))


if __name__ == '__main__':
    unittest.main()
