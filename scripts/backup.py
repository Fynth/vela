#!/usr/bin/env python3
"""Consistent SQLite snapshot and immutable media backup; restore only into an empty directory."""
from contextlib import closing
import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import sqlite3
import tarfile
import tempfile


def digest(path):
    h = hashlib.sha256()
    with path.open('rb') as f:
        while chunk := f.read(128 * 1024):
            h.update(chunk)
    return h.hexdigest()


def allowed(name):
    return name == 'vela.db' or bool(re.fullmatch(r'(media/[a-f0-9]{64}\.audio|artwork/[a-f0-9]{64}\.jpg)', name))


def create(source, output):
    if output.exists():
        raise ValueError('Backup destination already exists; select a new filename')
    source = source.resolve()
    with tempfile.TemporaryDirectory(prefix='vela-backup-') as temporary:
        temporary = Path(temporary)
        snapshot = temporary / 'vela.db'
        with closing(sqlite3.connect((source / 'vela.db').as_uri() + '?mode=ro', uri=True)) as db, closing(sqlite3.connect(snapshot)) as copy:
            db.backup(copy)
            if copy.execute('PRAGMA integrity_check').fetchone()[0] != 'ok' or copy.execute('PRAGMA foreign_key_check').fetchall():
                raise ValueError('Snapshot integrity check failed')
            names = ['vela.db'] + [r[0] for r in copy.execute('SELECT storage_path FROM media_assets UNION SELECT storage_path FROM artwork')]
        manifest = {'format': 1, 'files': {}}
        for name in names:
            if not allowed(name):
                raise ValueError('Snapshot contains an invalid storage path')
            path = snapshot if name == 'vela.db' else source / name
            if path.is_symlink() or not path.is_file():
                raise ValueError('Snapshot references missing or unsafe media')
            manifest['files'][name] = {'size': path.stat().st_size, 'sha256': digest(path)}
        manifest_path = temporary / 'manifest.json'
        manifest_path.write_text(json.dumps(manifest, indent=2))
        output.parent.mkdir(parents=True, exist_ok=True)
        part = output.parent / (output.name + '.partial')
        # Exclusive creation and no-clobber link preserve existing archives.
        with part.open('xb') as raw:
            with tarfile.open(fileobj=raw, mode='w:gz') as archive:
                archive.add(manifest_path, arcname='manifest.json', recursive=False)
                for name in names:
                    archive.add(snapshot if name == 'vela.db' else source / name, arcname=name, recursive=False)
            raw.flush()
            os.fsync(raw.fileno())
        try:
            os.link(part, output)
        finally:
            part.unlink()
    print('Backup created:', output)


def restore(archive_path, destination, max_bytes):
    destination = destination.resolve()
    if destination.exists() and (not destination.is_dir() or any(destination.iterdir())):
        raise ValueError('Restore destination must be a new or empty directory')
    destination.parent.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='vela-restore-', dir=destination.parent) as temporary:
        temporary = Path(temporary)
        with tarfile.open(archive_path, 'r:gz') as archive:
            entries = []
            total_declared = 0
            for member in archive:
                total_declared += member.size
                if member.size < 0 or total_declared > max_bytes or len(entries) >= 2000002:
                    raise ValueError('Backup exceeds restore budget or entry limit')
                entries.append(member)
            by_name = {m.name: m for m in entries}
            if len(entries) > 2000002 or len(entries) != len(by_name) or 'manifest.json' not in by_name:
                raise ValueError('Invalid or excessive backup entries')
            metadata = by_name['manifest.json']
            if not metadata.isfile() or metadata.size > 256 * 1024 * 1024:
                raise ValueError('Invalid backup manifest')
            manifest = json.load(archive.extractfile(metadata))
            if manifest.get('format') != 1:
                raise ValueError('Unsupported backup format')
            expected = manifest.get('files', {})
            if not isinstance(expected, dict) or 'vela.db' not in expected or set(by_name) != set(expected) | {'manifest.json'}:
                raise ValueError('Backup manifest and entries do not match')
            total = sum(m.size for m in entries)
            if total > max_bytes or total > shutil.disk_usage(destination.parent).free:
                raise ValueError('Backup exceeds restore budget or free disk space')
            for name, spec in expected.items():
                member = by_name[name]
                if not allowed(name) or not member.isfile() or member.size != spec['size'] or member.size < 0:
                    raise ValueError('Unsafe or inconsistent backup entry')
                path = temporary.joinpath(*PurePosixPath(name).parts)
                path.parent.mkdir(parents=True, exist_ok=True)
                with archive.extractfile(member) as source, path.open('xb') as target:
                    shutil.copyfileobj(source, target, length=128 * 1024)
                if path.stat().st_size != spec['size'] or digest(path) != spec['sha256']:
                    raise ValueError('Backup checksum mismatch')
        with closing(sqlite3.connect((temporary / 'vela.db').as_uri() + '?mode=ro', uri=True)) as db:
            if db.execute('PRAGMA integrity_check').fetchone()[0] != 'ok' or db.execute('PRAGMA foreign_key_check').fetchall():
                raise ValueError('Restored database failed integrity checks')
            referenced = {r[0] for r in db.execute('SELECT storage_path FROM media_assets UNION SELECT storage_path FROM artwork')}
            if not referenced.issubset(expected):
                raise ValueError('Database references media outside the backup')
        destination.mkdir(exist_ok=True, mode=0o700)
        for path in temporary.iterdir():
            shutil.move(str(path), destination / path.name)
    print('Backup restored:', destination)


def main():
    os.umask(0o077)
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest='action', required=True)
    c = sub.add_parser('create')
    c.add_argument('--data-dir', type=Path, required=True)
    c.add_argument('--output', type=Path, required=True)
    r = sub.add_parser('restore')
    r.add_argument('--archive', type=Path, required=True)
    r.add_argument('--data-dir', type=Path, required=True)
    r.add_argument('--max-bytes', type=int, default=20 * 1024**3)
    args = parser.parse_args()
    if args.action == 'create':
        create(args.data_dir, args.output)
    else:
        restore(args.archive, args.data_dir, args.max_bytes)


if __name__ == '__main__':
    main()
