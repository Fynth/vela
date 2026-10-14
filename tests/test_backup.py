"""Backup integrity and malicious-archive regressions using isolated real SQLite."""
from contextlib import closing
import importlib.util
import io
import json
from pathlib import Path
import sqlite3
import tarfile
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location('vela_backup', ROOT / 'scripts/backup.py')
backup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(backup)


class Backups(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.root = Path(self.tmp.name)
        self.data = self.root / 'source'
        self.data.mkdir()
        with closing(sqlite3.connect(self.data / 'vela.db')) as db, db:
            db.executescript((ROOT / 'migrations/0001_initial.sql').read_text())
            db.execute("INSERT INTO users(id,username,password_hash,is_admin) VALUES('u','admin','test',1)")
            self.name = 'media/' + 'a' * 64 + '.audio'
            db.execute('INSERT INTO media_assets VALUES(?,?,?,?,?)', ('a', 'a' * 64, self.name, 'audio/flac', 4))
        (self.data / 'media').mkdir()
        (self.data / self.name).write_bytes(b'fLaC')

    def tearDown(self):
        self.tmp.cleanup()

    def create(self):
        path = self.root / 'backup.tar.gz'
        backup.create(self.data, path)
        return path

    def repack(self, path, mutate):
        with tarfile.open(path) as old:
            entries = [(m, old.extractfile(m).read()) for m in old.getmembers()]
        with tarfile.open(path, 'w:gz') as new:
            for info, content in mutate(entries):
                info.size = len(content)
                new.addfile(info, io.BytesIO(content))

    def test_round_trip_and_no_overwrite(self):
        path = self.create()
        dest = self.root / 'restored'
        backup.restore(path, dest, 10 * 1024 * 1024)
        self.assertEqual((dest / self.name).read_bytes(), b'fLaC')
        with closing(sqlite3.connect(dest / 'vela.db')) as db:
            self.assertEqual(db.execute('SELECT username FROM users').fetchone(), ('admin',))
        with self.assertRaises(ValueError):
            backup.restore(path, dest, 10 * 1024 * 1024)
        with self.assertRaises(ValueError):
            backup.create(self.data, path)

    def test_corrupted_media_is_rejected_without_partial_restore(self):
        path = self.create()
        self.repack(path, lambda es: [(m, b'FAIL' if m.name == self.name else c) for m, c in es])
        dest = self.root / 'restored'
        with self.assertRaisesRegex(ValueError, 'checksum'):
            backup.restore(path, dest, 10 * 1024 * 1024)
        self.assertFalse(dest.exists())

    def test_duplicate_and_traversal_members_are_rejected(self):
        path = self.create()
        self.repack(path, lambda es: es + [es[-1]])
        with self.assertRaisesRegex(ValueError, 'entries'):
            backup.restore(path, self.root / 'restored', 10 * 1024 * 1024)
        path.unlink()
        path = self.create()
        def traversal(es):
            meta = tarfile.TarInfo('../escape')
            return es + [(meta, b'evil')]
        self.repack(path, traversal)
        with self.assertRaises(ValueError):
            backup.restore(path, self.root / 'restored', 10 * 1024 * 1024)
        self.assertFalse((self.root / 'escape').exists())

    def test_disk_budget_and_unsafe_source_are_rejected(self):
        path = self.create()
        with self.assertRaisesRegex(ValueError, 'budget'):
            backup.restore(path, self.root / 'restored', 1)
        (self.data / self.name).unlink()
        (self.data / self.name).symlink_to(self.data / 'vela.db')
        path.unlink()
        with self.assertRaisesRegex(ValueError, 'unsafe'):
            backup.create(self.data, path)

    def test_archive_symlink_is_rejected(self):
        path = self.create()
        def link(es):
            result = []
            for m, c in es:
                if m.name == self.name:
                    m.type = tarfile.SYMTYPE
                    m.linkname = '../../outside'
                result.append((m, c))
            return result
        self.repack(path, link)
        with self.assertRaisesRegex(ValueError, 'Unsafe'):
            backup.restore(path, self.root / 'restored', 10 * 1024 * 1024)


if __name__ == '__main__':
    unittest.main()
