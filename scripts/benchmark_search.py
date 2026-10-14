#!/usr/bin/env python3
"""Compare baseline and bounded-cardinality FTS plans on the same isolated database."""
import argparse
import json
from pathlib import Path
import tempfile
import time
from benchmark_catalog import seed_database, connect, sql_projection, catalog_query, summarize, OWNER, hardware


def adaptive(connection, expression):
    count = connection.execute('SELECT COUNT(*) FROM (SELECT rowid FROM tracks_fts WHERE tracks_fts MATCH ? LIMIT 4097)', (expression,)).fetchone()[0]
    if count == 0:
        return []
    if count <= 4096:
        projection = sql_projection().replace('FROM tracks t', 'FROM tracks_fts CROSS JOIN tracks t ON t.rowid=tracks_fts.rowid')
        query = projection + '? WHERE t.owner_id=? AND tracks_fts MATCH ? ORDER BY t.id ASC LIMIT 51'
        return connection.execute(query, (OWNER, OWNER, expression)).fetchall()
    query, parameters = catalog_query(sql_projection(), search=expression)
    return connection.execute(query, parameters).fetchall()


def main():
    p = argparse.ArgumentParser(description=__doc__)
    p.add_argument('--tracks', type=int, default=100000)
    p.add_argument('--output', type=Path, default=Path('/tmp/vela-search-comparison.json'))
    args = p.parse_args()
    if not 100 <= args.tracks <= 1000000:
        p.error('--tracks must be100..1000000')
    root = Path('target/catalog-bench')
    root.mkdir(parents=True, exist_ok=True)
    with tempfile.TemporaryDirectory(prefix='vela-search-', dir=root) as directory:
        database = Path(directory) / 'vela.db'
        seed_database(database, args.tracks)
        connection = connect(database)
        report = {'tracks': args.tracks, 'hardware': hardware(root), 'queries': {}}
        for name, expression in [('rare', '"UniqueNeedle"*'), ('broad', '"Signal"*'), ('absent', '"AbsentWordThatIsNotPresent"*')]:
            query, parameters = catalog_query(sql_projection(), search=expression)
            baseline = lambda: connection.execute(query, parameters).fetchall()
            changed = lambda: adaptive(connection, expression)
            if baseline() != changed():
                raise RuntimeError('Search results differ')
            result = {}
            for label, run in [('baseline', baseline), ('adaptive', changed)]:
                for _ in range(10):
                    run()
                samples = []
                for _ in range(100):
                    start = time.perf_counter_ns()
                    run()
                    samples.append((time.perf_counter_ns() - start) / 1e6)
                result[label] = summarize(samples)
                print(name, label, result[label], flush=True)
            report['queries'][name] = result
        connection.close()
        args.output.write_text(json.dumps(report, indent=2))


if __name__ == '__main__':
    main()
