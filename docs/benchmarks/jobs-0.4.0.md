# Synthetic job-history HTTP measurements

Executed 2026-10-16T09:02:44.794173+00:00 using native Vela0.4.0 on loopback. These are serial, warm authenticated task-history requests against synthetic metadata. No audio, catalog, streaming, concurrent-client capacity, browser rendering, federation or product-superiority claim is made.

CPU: AMD Ryzen 7 8845HS w/ Radeon 780M Graphics; 16 logical CPUs; memory 15649236 kB. OS: Linux-7.2.10-1-cachyos-x86_64-with-glibc2.44. Python 3.14.8; Python SQLite 3.54.0.

Native SHA256: `9193c5d99cdf30514d7a399380be21dc4212e013d7e6179e90146c2bafd14885`. Runtime source fingerprint: `27cad372b00081e50fb9d38ab0d4e21559725073d5c617372bef78ae742d3663`. Script SHA256: `bc41a3351f985738dcd647993fa4ab8b405e111030fb4a063d378250114debbc`. Full source hashes and raw samples are in the adjacent JSON.

The server creates/migrates a fresh private temporary database and real accounts through setup/admin APIs. It stops cleanly before deterministic synthetic import/scan histories are inserted, then restarts with saved fixture sessions. Each owner has equal import/scan counts; completed/failed/cancelled states cycle evenly. Pairs deliberately share UUIDs and timestamps across tables. An equally large foreign history tests isolation. All jobs are terminal: workers perform no downloads or scans. No existing database or remote resource is accessed.

Timing uses one reused HTTP/1.1 connection and perf_counter_ns from request send through complete response-body read. Normal per-request session authentication is included; client JSON parsing/validation, fixture seeding, initial account setup/password login and startup are excluded. Each case has 5 discarded warmups and 100 retained requests. Percentiles use empirical nearest rank. Every response is checked against an exact owned SQL reference prepared before startup; reference OFFSET only selects benchmark anchors offline, while timed HTTP uses real keyset cursors. Warm OS/database caches and the default four-connection SQLx pool apply; no CPU pinning or cold-cache reset is used.

Shared-host workload during run: Shared development host. Production Chromium E2E and native restart/fault probes may overlap; no CPU isolation or capacity inference. Results describe this run; tail values at this sample size do not establish a service-level guarantee.

| Owned + foreign history | Case | p50 ms | p95 ms | p99 ms | Rows |
| --- | --- | ---: | ---: | ---: | ---: |
| 10,000 + 10,000 | first_page | 0.451 | 0.560 | 0.690 | 50 |
| 10,000 + 10,000 | middle_cursor | 0.474 | 0.761 | 1.129 | 50 |
| 10,000 + 10,000 | deep_99_percent_cursor | 0.478 | 0.840 | 0.905 | 50 |
| 10,000 + 10,000 | completed_filter | 0.462 | 0.721 | 1.204 | 50 |
| 10,000 + 10,000 | failed_filter | 0.484 | 0.921 | 1.790 | 50 |
| 10,000 + 10,000 | cancelled_filter | 0.498 | 0.796 | 1.097 | 50 |
| 10,000 + 10,000 | imports_filter | 0.309 | 0.455 | 0.522 | 50 |
| 10,000 + 10,000 | scans_filter | 0.369 | 0.501 | 0.630 | 50 |
| 10,000 + 10,000 | deep_95_percent_failed_filter | 0.508 | 0.659 | 1.061 | 50 |
| 100,000 + 100,000 | first_page | 0.467 | 0.734 | 0.877 | 50 |
| 100,000 + 100,000 | middle_cursor | 0.485 | 0.654 | 0.984 | 50 |
| 100,000 + 100,000 | deep_99_percent_cursor | 0.443 | 0.608 | 0.687 | 50 |
| 100,000 + 100,000 | completed_filter | 0.486 | 0.810 | 1.013 | 50 |
| 100,000 + 100,000 | failed_filter | 0.493 | 0.866 | 1.494 | 50 |
| 100,000 + 100,000 | cancelled_filter | 0.503 | 0.711 | 0.978 | 50 |
| 100,000 + 100,000 | imports_filter | 0.305 | 0.559 | 0.601 | 50 |
| 100,000 + 100,000 | scans_filter | 0.369 | 0.533 | 0.676 | 50 |
| 100,000 + 100,000 | deep_95_percent_failed_filter | 0.504 | 0.742 | 1.881 | 50 |

Native SQLite last-writer versions read from the native-created database header before any Python seed: 3.46.0. This identifies the native SQLite writer independently of the Python SQLite seeder.

Reproduce from this source checkpoint with the matching already-built release binary:

```bash
python3 scripts/benchmark_jobs.py --binary target/release/vela-server --counts 10000 100000 --samples 100 --warmups 5 \
  --json docs/benchmarks/jobs-0.4.0.json --markdown docs/benchmarks/jobs-0.4.0.md \
  --shared-host-workload 'Describe concurrent host activity for the new run'
```
