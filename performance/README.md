# Urocissa performance benchmark

This is a local, disposable benchmark for measuring database insert, restart/readiness, real-browser loading, scrolling, and metadata editing before fixture deletion. It creates deterministic metadata-only records. Playwright serves a transparent placeholder for `/object/**`, so album and media controls are rendered without reading real media files.

## Run it

From the repository root (PowerShell):

```powershell
.\performance\run.ps1 smoke                 # 1,000 records; quick harness check
.\performance\run.ps1 baseline              # 100,000 records; three samples
.\performance\run.ps1 compare               # new three-sample run vs baseline
```

Useful options can be passed to the Node runner directly:

```powershell
node performance\run.mjs baseline --count 100000 --samples 3 --seed 20260718
node performance\run.mjs compare --baseline .performance\baseline\latest\summary.json --headed
node performance\run.mjs smoke --count 100000 --samples 1
```

The runner builds the backend with `--profile dev-release --features performance-test` (never plain `--release`), builds the frontend, starts an isolated server, inserts the fixture, and restarts it to measure recovery. Chromium then performs the existing login/top-scroll/middle-jump/end-jump journey followed by:

- creating and titling an album;
- creating, updating, and deleting a share;
- editing one media item's description, tags, album membership, favorite/archive state, and trash state, restoring temporary changes;
- selecting all items shown by Home and applying/removing a marker tag, favorite/archive changes, album membership, and trash/restore operations;
- entering the test album and setting a cover; and
- warm reloading and auditing the persisted final state.

Every timed UI edit measures RAM publication: it waits for its target response, application API quiet, and the backend barrier without forcing Redb durability. Logical audits run outside those intervals. A separate `write-behind-drain` phase measures Redb materialization and requires logical/disk audits to match before warm reload. The runner then injects a one-shot write-behind chunk failure, terminates the process before retry, restarts it, and verifies the exact committed marker prefix from Redb. The fixture is finally deleted by the restarted process, and both disk and memory must contain zero records. Use `UROCISSA_PERF_SKIP_BUILD=1` only when the dev-release backend and frontend have already been rebuilt for the current source.

The artifact schema is version 7 and includes the complete RAM-first edit and write-behind workload, the selected 8,192-record flush chunk size, and fixed Redb cache budgets in `environment`: 256 MiB main, 32 MiB tree snapshot, 16 MiB query snapshot, 8 MiB expire, and 128 MiB migration source/repair (the migration destination uses the 256 MiB main policy). The report records 100 ms sampled current/global/phase peak RSS, cache usage/evictions/hits/misses, and estimated arena/index/query/snapshot/write-behind component memory. Derived tree/query snapshots remain on internal schema 6: one fixed-width ordinal/selection bitmap blob per tree snapshot plus cached scrollbar boundaries and structural-epoch validation. Incompatible temp/query caches are deleted at startup. Schema 6 and older benchmark baselines are intentionally incompatible with comparison runs. Generate a fresh schema 7 baseline after upgrading:

```powershell
.\performance\run.ps1 baseline
```

## Results

Reports are written under `.performance/` (ignored by Git):

- `baseline/latest/summary.json` and `report.md` contain the saved reference.
- `results/<timestamp>/` contains comparison output and raw JSONL backend events/logs.
- `smoke/<timestamp>/` contains quick-run artifacts and a browser screenshot.

The report includes median, p95, and max timings for the fixture API, restart, every read/edit browser phase, HTTP method and route latency for GET/POST/PUT/DELETE requests, deletion, and structured backend operations such as `get_data.read_range`, `prefetch.filter_items`, and `tree_snapshot.flush_disk`. Phase status metrics also expose pending/active/flushing record work, estimated drain time, recent flush records/chunks, and EWMA flush throughput. Long tasks, frame gaps, paints, and heap readings are reset for each phase instead of accumulating across the journey. A comparison marks a timing as notable when it is at least 10% slower and exceeds the absolute noise floor (10 ms for browser phases, 1 ms for server operations). Correctness failures always fail the run independently of timing thresholds.

The benchmark-only fixture, barrier, drain, phase, status, audit, and restart-probe routes are protected by a per-run `X-Urocissa-Perf-Token` and only operate when `UROCISSA_PERF_ROOT` points to a directory containing `.urocissa-performance-root`. `POST /__perf/audit` accepts `view: "logical" | "disk"`; `POST /__perf/restart-probe` is the isolated failure-injection entry point. These routes are compiled only with the `performance-test` feature, so production builds do not expose them.
