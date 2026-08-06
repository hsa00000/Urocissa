# Hybrid virtual-scroll testing contract

This document is the regression contract for Urocissa's hybrid virtual scroll. It is not a
general scrolling checklist: every blocking rule below protects a coordinate, projection, or
native-input property that the implementation depends on.

## Invocation policy: explicit request only

This gate is deliberately **not part of ordinary validation**. Both profiles occupy an
interactive Windows desktop, drive real browser input, and produce traces and screenshots; a
typical local `Quick` run takes about one to two minutes and a `Full` run can take roughly nine
minutes, depending on the machine.

- Do not run `Quick`, `Full`, or the equivalent hybrid browser scenarios merely because a change
  touches scroll code, because a merge or release is approaching, or because another general
  checklist asks for visual verification.
- Run them only when the user explicitly requests hybrid-scroll testing, the scroll performance
  gate, complete trusted-input/visual validation, or equivalent wording in the current request.
- Without that request, perform only proportionate fast checks: for example `cargo test` for Rust,
  a targeted Vitest file or focused type check for frontend logic, or syntax/diff checks for
  documentation and harness-only edits.
- Record `Hybrid scroll gate not run (not explicitly requested)` in the handoff. That status is
  expected, not a failed or missing gate.

Invoking the runner manually is itself an explicit choice, so the script does not add an
interactive confirmation prompt. The policy exists to prevent agents and routine workflows from
starting the expensive browser matrix automatically.

## What must never change accidentally

The controller owns one logical position and three physical projections:

- `V = p - O` at every observable checkpoint.
- `native-top` uses `O = 0`.
- `compensated` keeps the physical viewport at the configured compensation anchor.
- `native-bottom` uses `O = P - U`, where `P` is measured from the browser rather than inferred
  from logical content height.
- A threshold crossing changes mode and transition generation exactly once and performs exactly
  one transition `scrollTop` write. If native inertia continues after entering `compensated`, its
  later real `scrollend` may perform the normal compensated re-anchor; that is recorded separately
  as `postTransitionInternalWriteDelta`.
- Repeated outward input at a native boundary performs zero controller writes.
- A row that remains rendered may move because `V` changed, but projection rebasing must not add
  another visual movement.

For adjacent animation frames containing the same anchor row, projection continuity is measured
as:

```text
residual = abs((anchorTopNow - anchorTopBefore) + (logicalTopNow - logicalTopBefore))
```

The logical movement cancels the expected screen movement. The remaining residual is the actual
projection jump and must be at most 1 CSS pixel across every mode or generation transition.

## Blocking gates and advisory evidence

| Contract | Blocking threshold |
| --- | ---: |
| Handoff projection residual | `<= 1 CSS px` |
| Handoff animation-frame gap | `< 25 ms` |
| Trusted input to first forward visual movement | `< 25 ms` |
| Scroll plus scrollend work | `< 0.7 ms/event` |
| Long tasks | `0` |
| Reverse frames during a directional gesture | `0` |
| Internal writes per mode handoff | `1` |
| Controller writes during pure native boundary input | `0` |
| Bottom origin error after settle | `<= 1 px` |
| Blank projection or horizontal overflow | `0 frames` |

Chrome can terminate the unfinished smooth portion of the wheel notch that causes a physical
rebase. Such a pulse is recorded as `truncatedHandoffPulse` and is advisory by itself. It is only
accepted when it moves forward without overshoot, satisfies the projection and responsiveness
gates, and the immediately following same-direction control pulse delivers its complete native
displacement. A truncated control pulse or two consecutive truncated pulses is blocking.

Do not use the old `visualDiscontinuityPx` interpretation. It represented the absence of reverse
movement, not a measured projection residual. New reports use
`handoffProjectionResidualPx`, `wheelDisplacementRatio`, and `truncatedHandoffPulse` separately.

## Test layers

1. **Pure metric tests** verify the residual and truncation formulas without Vue or a browser.
2. **Vue/Vitest state tests** verify thresholds, coordinate rebasing, internal events, short
   content, geometry reconciliation, layout caps, and zero writes at native boundaries.
3. **Browser geometry tests** verify real DOM projection, placeholders, mode sequences, errors,
   frame timing, and trace budgets.
4. **Trusted Windows input tests** use `SendInput`, `InitializeTouchInjection`, and
   `InjectTouchInput`. The helper resolves and verifies the target PID, window, foreground state,
   and screen-to-CSS calibration before sending input.
5. **Compositor evidence** compares same-run baseline, peak, and settled captures for native
   elastic overscroll. Golden screenshots are deliberately not used because Chrome rendering,
   fonts, DPI, and OS compositor output vary across machines.

The Vue controller remains the source of truth. Browser instrumentation is development-only and
must observe the controller; it must not repair state or make a scenario pass.

## Running the local gate

Requirements:

- Windows with an interactive desktop and PowerShell 7.
- Google Chrome installed.
- Existing `gallery-frontend` and `performance` dependencies already installed.
- The intended backend listening at `127.0.0.1:5673`.
- The intended Vite frontend available at `http://localhost:5173`.
- A collection large enough to exercise the top and bottom handoffs.

The gate never installs dependencies and never starts or replaces Vite, Rocket, or another
long-running service. Browser scenarios launch short-lived isolated Chrome windows with temporary
profiles; they never attach to or replace the user's Chrome profile. The gate authenticates
through the Vite proxy as part of preflight and stops before testing if the password or either
endpoint is wrong.

Set the password only in the current shell:

```powershell
$env:UROCISSA_PASSWORD = Read-Host 'Urocissa password'
```

When the user explicitly requests a quick hybrid-scroll validation, run:

```powershell
.\performance\run-hybrid-scroll-gate.ps1 -Profile Quick
```

`Quick` runs TypeScript checking, the pure metric tests, targeted virtual-scroll Vitest files,
one trusted top handoff, one trusted bottom handoff, short content, and height-clamp projection.

When the user explicitly requests the complete gate, release-quality scroll validation, trusted
touch/elastic verification, or the complete device/theme matrix, run:

```powershell
.\performance\run-hybrid-scroll-gate.ps1 -Profile Full
```

`Full` runs all frontend unit tests, lint, and production build sequentially, followed by:

- top and bottom handoffs in desktop light and dark themes, three samples each;
- top and bottom native elastic touch, three samples each;
- bottom live-offset during elastic input, five samples;
- short all-native and 120-million-pixel height-cap projection;
- 390x844 Android/touch behavior and a separate 390px desktop/mouse context.

Do not move the mouse, type, change the foreground window, alter display scaling, or lock the
desktop while a headed native-input scenario is active. The Windows helper aborts when it can
detect a target mismatch; avoiding competing input also keeps timing evidence meaningful.

## Artifacts and reading the result

Each run writes only ignored artifacts under:

```text
performance/.performance/hybrid-scroll/<timestamp>-<profile>/
```

The directory contains:

- `summary.md`: the first file to inspect; it lists every stage and the key handoff metrics.
- `gate-report.json`: versioned gate contract, environment, Git SHA/dirty state, commands, stage
  results, and scenario summaries.
- `logs/`: complete stdout/stderr for static checks and scenarios.
- `scenarios/<name>/report.json`: the full per-frame/event/browser report with the password
  redacted.
- `scenarios/<name>/traces/`: one complete gzip-compressed Chrome JSON trace per sample.
- `scenarios/<name>/checkpoints/`: baseline, transition, peak, and settled CDP/Windows captures
  applicable to that scenario.

The process exits nonzero if any blocking rule fails. Infrastructure failures stop immediately;
after preflight, test failures are collected across the remaining scenarios so one run produces a
complete diagnostic summary.

Chrome traces are intentionally complete and stored as `.json.gz`; screenshots can still make a
Full run sizeable. Keep the most recent passing Full report and any report needed for
investigation; remove older ignored run directories manually after they are no longer useful. The
gate never deletes prior evidence.

For a handoff failure, inspect in this order:

1. Confirm `modeSequence`, `transitionGenerationDelta`, and
   `transitionInternalWriteDelta`. Use the total deltas only to diagnose legitimate later
   compensated settlement.
2. Inspect `transitionFrames` and `handoffProjectionResidualPx`.
3. Check `reverseMovementFrameCount` and `inputToFirstVisualMotionMs`.
4. Treat `truncatedHandoffPulse` as advisory, then verify the corresponding control pulse has
   `wheelDisplacementErrorPx <= 1`.
5. Open the transition screenshots and trace only after the numerical contract identifies the
   failing interval.

For a bottom failure, additionally inspect `P`, `U`, `O`, `bottomOriginError`, total-height
revision, and write count. Never diagnose bottom behavior by assuming physical and logical height
are equal.

A frame-gap-only failure with low event work, no long task, and no scroll event inside the gap is
usually desktop scheduling noise rather than controller work, but it still leaves that run failed.
Close competing foreground/background activity and run the complete selected profile again. Keep
both reports: the new passing report is the required gate, while the failed report preserves the
environmental evidence. Do not weaken the 25 ms threshold or delete a failed row from the summary.

## Choosing a profile after an explicit request

| Requested validation | Profile |
| --- | --- |
| No explicit hybrid-scroll test request | Do not run either profile; use ordinary fast checks |
| Quick hybrid-scroll check or a focused handoff regression check | `Quick` |
| Full/complete/release-quality scroll validation | `Full` |
| Trusted touch, elastic overscroll, or the device/theme visual matrix | `Full` plus manual review of peak/settled captures |
| Validation after a browser/Chromium major-version upgrade | `Full` and compare advisory truncation counts with the previous local report |

The changed files never select a profile on their own. They can inform which profile to recommend,
but execution still requires the user's explicit request.

Real Android Chrome and iOS Safari remain valuable final spot checks for platform-specific spring
appearance. They complement this contract but do not replace the repeatable Windows trusted-input
and geometry gates.
