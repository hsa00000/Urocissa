# Urocissa performance benchmark

This is a local, disposable benchmark for measuring database insert, restart/readiness, real-browser loading, scrolling, and metadata editing before fixture deletion. It creates deterministic metadata-only records. Playwright serves a transparent placeholder for `/object/**`, so album and media controls are rendered without reading real media files.

## Run it

From the repository root (PowerShell):

```powershell
.\performance\run.ps1 smoke                 # 1,000 records; quick harness check
.\performance\run.ps1 baseline              # 100,000 records; three samples
.\performance\run.ps1 compare               # new three-sample run vs baseline
.\performance\run.ps1 storage               # 1,000,000-record V5/V6 storage gate
```

Useful options can be passed to the Node runner directly:

```powershell
node performance\run.mjs baseline --count 100000 --samples 3 --seed 20260718
node performance\run.mjs compare --baseline .performance\baseline\latest\summary.json --headed
node performance\run.mjs smoke --count 100000 --samples 1
node performance\run.mjs storage --count 1000000 --samples 3
```

Set `UROCISSA_PERF_BACKEND` to compare a separately built backend binary.
Comparison runs normally return a non-zero exit code when the noise-floor timing
gate fails. For explicitly approved diagnostic runs, set
`UROCISSA_PERF_ALLOW_TIMING_JITTER=1` to keep those timing findings advisory;
correctness failures still fail the command.

On a Linux VPS, the storage-only harness is a single command and does not install
Playwright or build the frontend:

```bash
./performance/run.sh storage --count 1000000 --samples 3
```

It creates one disposable V5 fixture, measures the production V5→V6 migration,
runs V5 and V6 through the same TreeState builder, and then launches the real
server once to measure complete readiness. The report includes storage-open and
O(1) count time, decode scans, TreeState time, records/s, peak RSS, redb cache
metrics, one normal-startup record iteration, one migration source scan, and zero
migration destination scans. The hard gates are V6 median ≤115% of V5 and peak
RSS ≤850 MiB. Results are written under `.performance/storage/`.

### Scroll-lag and hybrid virtual-scroll microbenchmark

With Rocket listening on `127.0.0.1:5673` and Vite on `127.0.0.1:5173`, run the
focused compensated-virtual-scroll benchmark with:

```powershell
node performance\scroll-lag.mjs --scenario continuous-down --samples 5 --expect strict-smooth
node performance\scroll-lag.mjs --scenario continuous-up --samples 5
node performance\scroll-lag.mjs --scenario worker-delay --worker-delay 300 --timer-zero-budget 10
node performance\scroll-lag.mjs --browser chrome --headed --scenario native-wheel `
  --samples 3 --pulses 20 --pulse-settle 400 --os-wheel-delta -120
```

The browser login password is mandatory and is never written to the report. Set it only for
the current shell (or pass `--password` explicitly):

> **Manual opt-in only:** the hybrid-scroll gates and the trusted Windows browser scenarios below
> are expensive and must not run as routine validation. Run them only when the user explicitly
> requests hybrid-scroll testing or equivalent complete scroll validation. Otherwise use the
> project's ordinary fast checks, such as `cargo test` or focused frontend tests.

```powershell
$env:UROCISSA_PASSWORD = Read-Host 'Urocissa password'

# Explicitly requested quick scroll-contract gate
.\performance\run-hybrid-scroll-gate.ps1 -Profile Quick

# Explicitly requested complete scroll-contract gate
.\performance\run-hybrid-scroll-gate.ps1 -Profile Full

node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario hybrid-top-handoff --headed --samples 3 --expect strict-smooth
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario hybrid-bottom-handoff --headed --samples 3 --expect strict-smooth
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario native-elastic-top --headed --samples 3 --expect strict-smooth `
  --checkpoint-dir performance\.performance\native-elastic-top
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario native-elastic-bottom --headed --samples 3 --expect strict-smooth `
  --checkpoint-dir performance\.performance\native-elastic-bottom
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario hybrid-bottom-live-offset --headed --samples 5 --expect strict-smooth `
  --checkpoint-dir performance\.performance\hybrid-bottom-live-offset
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario short-all-native --samples 1 --expect strict-smooth
node performance\scroll-lag.mjs --url http://localhost:5173 `
  --scenario height-clamp-projection --samples 1 --expect strict-smooth
```

When explicitly requested, the two gate profiles perform environment/authentication preflight, run frontend checks
sequentially, execute the required scenario matrix, and create one Markdown/JSON summary under
`performance/.performance/hybrid-scroll/`. They never install dependencies or start services.
The complete invariants, blocking thresholds, artifact layout, known Chrome handoff-pulse
truncation advisory, and failure-triage procedure are documented in
[`docs/HYBRID_VIRTUAL_SCROLL_TESTING.md`](../docs/HYBRID_VIRTUAL_SCROLL_TESTING.md).

Hybrid-specific scenarios are `hybrid-top-handoff`, `hybrid-bottom-handoff`,
`hybrid-bottom-live-offset`, `native-elastic-top`, `native-elastic-bottom`,
`short-all-native`, and `height-clamp-projection`. The existing scenarios remain
`continuous-down`, `continuous-up`, `discrete-wheel`, `discrete-wheel-delay`,
`native-wheel`, `native-wheel-delay`, `worker-delay`, `bounds`, `scrollbar`, `locate`,
`resize`, and `mobile`. `bounds` alternates upper and lower bounds between samples;
`locate` can use `--locate <hash>` or a hash observed from the loaded page.
Reports include CDP timer install/fire counts for 0, 50, 75, and 100 ms timers. Use
`--expect strict-smooth` when every sample must pass the jank gate.
The browser instrumentation also records element `scrollend` events and waits for the
compensated physical buffer to return to its original anchor before finalizing a wheel sample.
The scroll-work budget includes both ordinary `scroll` handlers and the final `scrollend`
commit, so moving work to transaction completion cannot hide it from the jank gate.
For synthetic scenarios the budget remains normalized per input pulse. Native-wheel scenarios
are normalized per emitted scroll event because one real Chrome smooth-scroll notch produces
multiple compositor-driven scroll events; summing those animation frames into a per-notch
budget would report smooth native motion as handler jank.

`native-wheel` is the Windows hardware-path approximation used for the mouse-notch
regression. It launches the locally installed Chrome Stable channel with Playwright's
temporary user-data directory and a fresh browser context; it never connects to an existing
Chrome profile. `native-wheel-input.ps1` targets that isolated window by a unique title and
PID, verifies that it is the foreground window, and then sends one Windows
`MOUSEEVENTF_WHEEL` input per pulse (`-120` is one downward notch). The report keeps the raw
OS delta, Chrome's trusted DOM wheel delta and final `defaultPrevented` state, physical scroll
and `scrollend` events, per-frame row displacement, and final physical-anchor error.
`native-wheel-delay` adds the configured row-response delay.

The hybrid handoff scenarios use the same Windows `SendInput(MOUSEEVENTF_WHEEL)` path and
assert the exact `native-top → compensated → native-top` or
`native-bottom → compensated → native-bottom` sequence, one-pixel projection continuity,
no reverse frame, and the final `p/P/O/V/U` invariants. The elastic scenarios use
`InitializeTouchInjection` and `InjectTouchInput`; every down/update/up must arrive as a
trusted, non-cancelled DOM touch event. Because Chromium's Windows build keeps elastic
overscroll behind feature flags, these isolated test windows enable Chromium's native
`ElasticOverscroll` and `OverscrollEffectOnNonRootScrollers` features. This changes only the
test browser's native compositor behavior; the application contains no simulated spring.

With `--checkpoint-dir`, the Windows helper records GDI `CopyFromScreen` frames and a
composed-window baseline/peak/settled triplet captured through the Windows screenshot path,
while Playwright records the matching CDP screenshot and DOM geometry. The visual gate
requires a measurable native peak, a fully recovered settled frame, zero controller writes
at a pure boundary, and at most one geometry-reconciliation write for the live-offset case.
The helper preserves and restores the user's clipboard after composed-window capture.

The runner builds the backend with `--profile dev-release --features performance-test` (never plain `--release`), builds the frontend, starts an isolated server, inserts the fixture, and restarts it to measure recovery. Chromium then performs the existing login/top-scroll/middle-jump/end-jump journey followed by:

- creating and titling an album;
- creating, updating, and deleting a share;
- editing one media item's description, tags, album membership, favorite/archive state, and trash state, restoring temporary changes;
- selecting all items shown by Home and applying/removing a marker tag, favorite/archive changes, album membership, and trash/restore operations;
- entering the test album and setting a cover; and
- warm reloading and auditing the persisted final state.

Every timed UI edit measures RAM publication: it waits for its target response, application API quiet, and the backend barrier without forcing Redb durability. Logical audits run outside those intervals. A separate `write-behind-drain` phase measures Redb materialization and requires logical/disk audits to match before warm reload. The runner then injects a one-shot write-behind chunk failure, terminates the process before retry, restarts it, and verifies the exact committed marker prefix from Redb. The fixture is finally deleted by the restarted process, and both disk and memory must contain zero records. Use `UROCISSA_PERF_SKIP_BUILD=1` only when the dev-release backend and frontend have already been rebuilt for the current source. Set `UROCISSA_PERF_DETAILED_TIMING=1` only for profiling; it adds `write_behind.flush.targets.decode`, `.overlay`, `.encode_insert`, and `.commit` events and is intentionally excluded from formal timing comparisons.

The artifact schema is version 7 and includes the complete RAM-first edit and write-behind workload, the selected 8,192-record flush chunk size, and fixed Redb cache budgets in `environment`: 128 MiB main, 32 MiB tree snapshot, 16 MiB query snapshot, 8 MiB expire, and 128 MiB migration source/repair (the migration destination uses the 128 MiB main policy). The report records 100 ms sampled current/global/phase peak RSS, cache usage/evictions/hits/misses, and estimated arena/index/query/snapshot/write-behind component memory. Derived tree/query snapshots remain on internal schema 6: one fixed-width ordinal/selection bitmap blob per tree snapshot plus cached scrollbar boundaries and structural-epoch validation. Incompatible temp/query caches are deleted at startup. Schema 6 and older benchmark baselines are intentionally incompatible with comparison runs. Generate a fresh schema 7 baseline after upgrading:

```powershell
.\performance\run.ps1 baseline
```

## Results

Reports are written under `.performance/` (ignored by Git):

- `baseline/latest/summary.json` and `report.md` contain the saved reference.
- `results/<timestamp>/` contains comparison output and raw JSONL backend events/logs.
- `smoke/<timestamp>/` contains quick-run artifacts and a browser screenshot.

The report includes median, p95, and max timings for the fixture API, restart, every read/edit browser phase, HTTP method and route latency for GET/POST/PUT/DELETE requests, deletion, and structured backend operations such as `get_data.read_range`, `prefetch.filter_items`, and `tree_snapshot.flush_disk`. Phase status metrics also expose pending/active/flushing record work, estimated drain time, recent flush records/chunks, and EWMA flush throughput. Long tasks, frame gaps, paints, and heap readings are reset for each phase instead of accumulating across the journey. A comparison fails when a timing regression exceeds the absolute noise floor (10 ms for browser phases, 1 ms for server operations); there is no additional percentage allowance. Correctness failures always fail the run independently of timing thresholds.

`optimization-gate.mjs` combines comparison, storage, and one or more
`edit-memory.mjs` artifacts into an ignored `optimization-gate.json`. Pass
`--timing-advisory` only when timing jitter has been explicitly waived; the
artifact still records every regression while correctness, schema, cache, and
memory checks remain blocking.

The benchmark-only fixture, barrier, drain, phase, status, audit, and restart-probe routes are protected by a per-run `X-Urocissa-Perf-Token` and only operate when `UROCISSA_PERF_ROOT` points to a directory containing `.urocissa-performance-root`. `POST /__perf/audit` accepts `view: "logical" | "disk"`; `POST /__perf/restart-probe` is the isolated failure-injection entry point. These routes are compiled only with the `performance-test` feature, so production builds do not expose them.
