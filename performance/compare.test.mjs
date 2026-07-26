import assert from 'node:assert/strict'
import test from 'node:test'

import { compareSummaries } from './compare.mjs'

function summary(aggregates) {
  return {
    schemaVersion: 7,
    aggregates: Object.fromEntries(
      Object.entries(aggregates).map(([key, median]) => [key, { median }])
    )
  }
}

test('server timings only fail above the one millisecond noise floor', () => {
  const baseline = summary({ 'backend.tree.rebuild.ms': 100 })
  assert.equal(
    compareSummaries(baseline, summary({ 'backend.tree.rebuild.ms': 101 })).passed,
    true
  )
  assert.equal(
    compareSummaries(baseline, summary({ 'backend.tree.rebuild.ms': 101.001 })).passed,
    false
  )
})

test('browser timings only fail above the ten millisecond noise floor', () => {
  const baseline = summary({ 'browser.prefetch.wallMs': 100 })
  assert.equal(
    compareSummaries(baseline, summary({ 'browser.prefetch.wallMs': 110 })).passed,
    true
  )
  assert.equal(
    compareSummaries(baseline, summary({ 'browser.prefetch.wallMs': 110.001 })).passed,
    false
  )
})

test('memory and count metrics are reported but not treated as timing regressions', () => {
  const baseline = summary({
    'backend.stage.startup.rssBytes': 100,
    'backend.phase.edit.pendingRecords': 1
  })
  const comparison = compareSummaries(
    baseline,
    summary({
      'backend.stage.startup.rssBytes': 1_000,
      'backend.phase.edit.pendingRecords': 100
    })
  )
  assert.equal(comparison.passed, true)
})

test('phase-scoped async diagnostics remain visible without duplicating global gates', () => {
  const baseline = summary({
    'backend.phase.edit.write_behind.flush.batch.ms': 9,
    'backend.phase.edit.flushRecordsPerSecond': 1_000
  })
  assert.equal(
    compareSummaries(
      baseline,
      summary({
        'backend.phase.edit.write_behind.flush.batch.ms': 9,
        'backend.phase.edit.flushRecordsPerSecond': 100
      })
    ).passed,
    true
  )
})

test('incompatible schemas are rejected', () => {
  assert.throws(
    () => compareSummaries(summary({}), { schemaVersion: 6, aggregates: {} }),
    /incompatible benchmark schemas/
  )
})
