export function compareSummaries(baseline, current) {
  if (baseline.schemaVersion !== current.schemaVersion) {
    throw new Error(
      `incompatible benchmark schemas: baseline=${baseline.schemaVersion}, current=${current.schemaVersion}`
    )
  }

  const keys = new Set([
    ...Object.keys(baseline.aggregates ?? {}),
    ...Object.keys(current.aggregates ?? {})
  ])
  const metrics = {}
  for (const key of keys) {
    const before = baseline.aggregates?.[key]?.median ?? null
    const after = current.aggregates?.[key]?.median ?? null
    const comparable = Number.isFinite(before) && Number.isFinite(after)
    const delta = comparable ? after - before : null
    const relative = comparable && before !== 0 ? delta / before : null
    const timing =
      !key.startsWith('backend.phase.') &&
      /(?:\.ms|Ms)$/.test(key)
    const noiseFloorMs = timing
      ? key.startsWith('browser.')
        ? 10
        : 1
      : null
    const timingRegression = timing && delta != null && delta > noiseFloorMs
    const regression = timingRegression
    metrics[key] = {
      baseline: before,
      current: after,
      delta,
      relative,
      timing,
      noiseFloorMs,
      minimumThroughput: null,
      regression,
      notable: regression
    }
  }

  const notableRegressions = Object.entries(metrics)
    .filter(([, value]) => value.regression)
    .map(([key]) => key)
  return {
    policy: {
      serverNoiseFloorMs: 1,
      browserNoiseFloorMs: 10,
      timingRegressionAllowance: 'noise-floor-only'
    },
    metrics,
    notableRegressions,
    passed: notableRegressions.length === 0
  }
}
