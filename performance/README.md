# Urocissa performance benchmark

This is a local, disposable benchmark for measuring database insert, restart/readiness, real-browser loading and scrolling, and fixture deletion. It creates deterministic metadata-only records, so the 100,000-record run does not download or decode image files.

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
```

The runner builds the release backend with `--features performance-test`, builds the frontend, starts an isolated server, inserts the fixture, restarts it to measure recovery, drives Chromium through login/top-scroll/middle-jump/end-jump/reload, deletes the fixture, and verifies that disk and memory contain zero records afterward. Use `UROCISSA_PERF_SKIP_BUILD=1` when iterating on the runner after the binaries are already built.

## Results

Reports are written under `.performance/` (ignored by Git):

- `baseline/latest/summary.json` and `report.md` contain the saved reference.
- `results/<timestamp>/` contains comparison output and raw JSONL backend events/logs.
- `smoke/<timestamp>/` contains quick-run artifacts and a browser screenshot.

The report includes median, p95, and max timings for the fixture API, restart, each browser phase, deletion, and structured backend operations such as `get_data.read_range`, `prefetch.filter_items`, and `tree_snapshot.flush_disk`. A comparison marks a timing as notable when it is at least 10% slower and exceeds the absolute noise floor (10 ms for browser phases, 1 ms for server operations). Correctness failures always fail the run independently of timing thresholds.

The benchmark-only routes are protected by a per-run `X-Urocissa-Perf-Token` and only operate when `UROCISSA_PERF_ROOT` points to a directory containing `.urocissa-performance-root`; production builds do not expose the fixture routes.
