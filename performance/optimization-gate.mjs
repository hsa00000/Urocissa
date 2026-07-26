import { stat } from 'node:fs/promises'
import { readdir, readFile, writeFile } from 'node:fs/promises'
import { dirname, join, resolve } from 'node:path'

import { compareSummaries } from './compare.mjs'

const MIB = 1024 ** 2
const MAIN_CACHE_BYTES = 128 * MIB
const TREE_MEMORY_LIMIT_BYTES = 298_844_160
const RSS_ALLOWANCE_BYTES = 8 * MIB

const options = parseOptions(process.argv.slice(2))
const baselinePath = requiredPath(options.baseline, '--baseline')
const currentPath = requiredPath(options.current, '--current')
const previousPath = options.previous ? resolve(String(options.previous)) : null
const outputPath = resolve(
  String(options.output ?? join(dirname(currentPath), 'optimization-gate.json'))
)
const requireComplete = options['require-complete'] === true
const timingAdvisory = options['timing-advisory'] === true

const baseline = await readJson(baselinePath)
const current = await readJson(currentPath)
const previous = previousPath ? await readJson(previousPath) : null
const baselineStorage = options['baseline-storage']
  ? await readJson(resolve(String(options['baseline-storage'])))
  : null
const currentStorage = options['current-storage']
  ? await readJson(resolve(String(options['current-storage'])))
  : null
const baselineEdit = options['baseline-edit']
  ? await readEditInputs(String(options['baseline-edit']))
  : []
const currentEdit = options['current-edit']
  ? await readEditInputs(String(options['current-edit']))
  : []

const checks = []
const addCheck = (name, passed, details) => {
  checks.push({ name, passed, ...details })
}

const baselineComparison = compareSummaries(baseline, current)
addCheck('timings.vsBaseline', baselineComparison.passed || timingAdvisory, {
  advisory: timingAdvisory && !baselineComparison.passed,
  regressions: baselineComparison.notableRegressions
})
let previousComparison = null
if (previous) {
  previousComparison = compareSummaries(previous, current)
  addCheck('timings.vsPreviousCheckpoint', previousComparison.passed || timingAdvisory, {
    advisory: timingAdvisory && !previousComparison.passed,
    regressions: previousComparison.notableRegressions
  })
}

const baselineTreeBytes = aggregateMedian(
  baseline,
  'backend.stage.startup.component.tree.totalBytes'
)
const currentTreeBytes = aggregateMax(
  current,
  'backend.stage.startup.component.tree.totalBytes'
)
const treeLimit = Math.min(
  Number.isFinite(baselineTreeBytes) ? baselineTreeBytes : TREE_MEMORY_LIMIT_BYTES,
  TREE_MEMORY_LIMIT_BYTES
)
addCheck('memory.treeStateTotal', currentTreeBytes <= treeLimit, {
  actualBytes: currentTreeBytes,
  limitBytes: treeLimit
})

const cacheLimit = aggregateMax(
  current,
  'backend.stage.startup.redbCache.main.limitBytes'
)
const cacheUsed = aggregateMax(
  current,
  'backend.stage.startup.redbCache.main.usedBytes'
)
addCheck('memory.redbMainCacheLimit', cacheLimit === MAIN_CACHE_BYTES, {
  actualBytes: cacheLimit,
  expectedBytes: MAIN_CACHE_BYTES
})
addCheck('memory.redbMainCacheUsed', cacheUsed <= cacheLimit, {
  actualBytes: cacheUsed,
  limitBytes: cacheLimit
})

const baselineStartupRss = aggregateMedian(baseline, 'backend.stage.startup.rssBytes')
const currentStartupRss = aggregateMedian(current, 'backend.stage.startup.rssBytes')
addCheck(
  'memory.startupRss',
  currentStartupRss <= baselineStartupRss + RSS_ALLOWANCE_BYTES,
  {
    baselineBytes: baselineStartupRss,
    currentBytes: currentStartupRss,
    allowanceBytes: RSS_ALLOWANCE_BYTES
  }
)

if (baselineStorage && currentStorage) {
  addStorageChecks(checks, baselineStorage, currentStorage, timingAdvisory)
} else if (requireComplete) {
  addCheck('inputs.storage', false, {
    error: 'baseline and current storage artifacts are required'
  })
}

if (baselineEdit.length && currentEdit.length) {
  addEditChecks(checks, baselineEdit, currentEdit, timingAdvisory)
} else if (requireComplete) {
  addCheck('inputs.editMemory', false, {
    error: 'baseline and current edit-memory artifacts are required'
  })
}

const result = {
  schemaVersion: 1,
  generatedAt: new Date().toISOString(),
  policy: {
    serverNoiseFloorMs: 1,
    browserNoiseFloorMs: 10,
    timingAdvisory,
    rssAllowanceBytes: RSS_ALLOWANCE_BYTES,
    treeMemoryLimitBytes: TREE_MEMORY_LIMIT_BYTES,
    redbMainCacheBytes: MAIN_CACHE_BYTES
  },
  inputs: {
    baseline: baselinePath,
    previous: previousPath,
    current: currentPath,
    baselineStorage: options['baseline-storage'] ?? null,
    currentStorage: options['current-storage'] ?? null,
    baselineEdit: options['baseline-edit'] ?? null,
    currentEdit: options['current-edit'] ?? null
  },
  comparisons: {
    baseline: {
      passed: baselineComparison.passed,
      regressions: baselineComparison.notableRegressions
    },
    previous: previousComparison
      ? {
          passed: previousComparison.passed,
          regressions: previousComparison.notableRegressions
        }
      : null
  },
  checks,
  passed: checks.every((check) => check.passed)
}

await writeFile(outputPath, JSON.stringify(result, null, 2))
console.log(`optimization gate: ${result.passed ? 'PASS' : 'FAIL'}`)
for (const check of checks.filter((value) => !value.passed)) {
  console.log(`FAIL ${check.name}: ${JSON.stringify(check)}`)
}
console.log(`artifact: ${outputPath}`)
if (!result.passed) process.exitCode = 1

function addStorageChecks(target, before, after, timingAdvisory) {
  const add = (name, passed, details) => {
    target.push({ name: `storage.${name}`, passed, ...details })
  }
  add('schema', before.schemaVersion === after.schemaVersion, {
    baseline: before.schemaVersion,
    current: after.schemaVersion
  })
  add('records', before.records === after.records, {
    baseline: before.records,
    current: after.records
  })
  addTiming(
    add,
    'migrationWall',
    before.migrationWallMs,
    after.migrationWallMs,
    timingAdvisory
  )
  for (const field of [
    'storageOpenMs',
    'recordCountMs',
    'decodeScanMs',
    'treeStateWithDecodeMs',
    'treeStateBuildEstimateMs'
  ]) {
    addTiming(
      add,
      `v6.${field}`,
      median(before.v6.map((sample) => sample[field])),
      median(after.v6.map((sample) => sample[field])),
      timingAdvisory
    )
  }

  const baselineTreeMs = median(
    before.v6.map((sample) => sample.treeStateWithDecodeMs)
  )
  const currentThroughput = median(
    after.v6.map((sample) => sample.recordsPerSecond)
  )
  const minimumThroughput =
    before.records / ((baselineTreeMs + 1) / 1000)
  add('v6.throughput', currentThroughput >= minimumThroughput, {
    currentRecordsPerSecond: currentThroughput,
    minimumRecordsPerSecond: minimumThroughput
  })

  const baselinePeak = median(before.v6.map((sample) => sample.peakRssBytes))
  const currentPeak = median(after.v6.map((sample) => sample.peakRssBytes))
  add('v6.peakRss', currentPeak <= baselinePeak + RSS_ALLOWANCE_BYTES, {
    baselineBytes: baselinePeak,
    currentBytes: currentPeak,
    allowanceBytes: RSS_ALLOWANCE_BYTES
  })
  const currentCacheLimits = after.v6.map((sample) => sample.cache.limitBytes)
  const currentCacheUsed = after.v6.map((sample) => sample.cache.usedBytes)
  add(
    'v6.cacheLimit',
    currentCacheLimits.every((value) => value === MAIN_CACHE_BYTES),
    { actualBytes: currentCacheLimits, expectedBytes: MAIN_CACHE_BYTES }
  )
  add(
    'v6.cacheUsed',
    currentCacheUsed.every(
      (value, index) => value <= currentCacheLimits[index]
    ),
    { usedBytes: currentCacheUsed, limitBytes: currentCacheLimits }
  )
}

function addEditChecks(target, before, after, timingAdvisory) {
  const add = (name, passed, details) => {
    target.push({ name: `editMemory.${name}`, passed, ...details })
  }
  add('sampleCount', before.length >= 1 && after.length >= 1, {
    baseline: before.length,
    current: after.length
  })
  for (const field of [
    ['startupRss', (sample) => sample.startup.backend_rss_bytes],
    ['preEditRss', (sample) => sample.memory.preEditRssBytes],
    ['editPeakRss', (sample) => sample.memory.editPeakRssBytes]
  ]) {
    const baselineValue = median(before.map(field[1]))
    const currentValue = median(after.map(field[1]))
    add(field[0], currentValue <= baselineValue + RSS_ALLOWANCE_BYTES, {
      baselineBytes: baselineValue,
      currentBytes: currentValue,
      allowanceBytes: RSS_ALLOWANCE_BYTES
    })
  }
  addTiming(
    add,
    'editDrainWall',
    median(before.map((sample) => sample.workload.editWallMs)),
    median(after.map((sample) => sample.workload.editWallMs)),
    timingAdvisory
  )
  const baselineEditWall = median(
    before.map((sample) => sample.workload.editWallMs)
  )
  const baselineThroughput = median(
    before.map(
      (sample) => sample.completed.write_behind_flush_records_per_second
    )
  )
  const currentThroughput = median(
    after.map(
      (sample) => sample.completed.write_behind_flush_records_per_second
    )
  )
  const minimumThroughput =
    baselineThroughput * baselineEditWall / (baselineEditWall + 1)
  add('flushThroughput', currentThroughput >= minimumThroughput, {
    currentRecordsPerSecond: currentThroughput,
    minimumRecordsPerSecond: minimumThroughput
  })

  const databaseBytes = new Set(before.map((sample) => sample.fixture.databaseBytes))
  const expectedDatabaseBytes =
    databaseBytes.size === 1 ? [...databaseBytes][0] : null
  add(
    'databaseBytes',
    expectedDatabaseBytes != null &&
      after.every(
        (sample) => sample.fixture.databaseBytes === expectedDatabaseBytes
      ),
    {
      expectedBytes: expectedDatabaseBytes,
      currentBytes: after.map((sample) => sample.fixture.databaseBytes)
    }
  )
  add(
    'fixtureIdentity',
    after.every(
      (sample) =>
        sample.fixture.requestedRecords === before[0].fixture.requestedRecords &&
        sample.fixture.seed === before[0].fixture.seed
    ),
    {
      expectedRecords: before[0].fixture.requestedRecords,
      expectedSeed: before[0].fixture.seed
    }
  )

  const baselineTree = median(
    before.map((sample) => sample.preEdit.tree_memory.total_bytes)
  )
  const currentTree = Math.max(
    ...after.map((sample) => sample.preEdit.tree_memory.total_bytes)
  )
  const limit = Math.min(baselineTree, TREE_MEMORY_LIMIT_BYTES)
  add('treeStateTotal', currentTree <= limit, {
    actualBytes: currentTree,
    limitBytes: limit
  })
  add(
    'redbMainCache',
    after.every(
      (sample) =>
        sample.preEdit.redb_main_cache.limit_bytes === MAIN_CACHE_BYTES &&
        sample.preEdit.redb_main_cache.used_bytes <= MAIN_CACHE_BYTES
    ),
    {
      expectedLimitBytes: MAIN_CACHE_BYTES
    }
  )
}

function addTiming(add, name, baselineMs, currentMs, timingAdvisory) {
  const timingPassed = currentMs <= baselineMs + 1
  add(name, timingPassed || timingAdvisory, {
    advisory: timingAdvisory && !timingPassed,
    baselineMs,
    currentMs,
    noiseFloorMs: 1
  })
}

function aggregateMedian(summary, key) {
  return summary.aggregates?.[key]?.median ?? Number.NaN
}

function aggregateMax(summary, key) {
  return summary.aggregates?.[key]?.max ?? Number.NaN
}

function median(values) {
  const sorted = values.filter(Number.isFinite).toSorted((left, right) => left - right)
  if (!sorted.length) return Number.NaN
  return sorted[Math.floor(sorted.length / 2)]
}

async function readJson(path) {
  return JSON.parse(await readFile(path, 'utf8'))
}

async function readEditInputs(input) {
  const paths = input.split(',').map((value) => resolve(value.trim()))
  const expanded = []
  for (const path of paths) {
    const metadata = await stat(path)
    if (metadata.isDirectory()) {
      const entries = await readdir(path)
      expanded.push(
        ...entries
          .filter((entry) => /^v6-\d+\.json$/.test(entry))
          .sort()
          .map((entry) => join(path, entry))
      )
    } else {
      expanded.push(path)
    }
  }
  return Promise.all(expanded.map(readJson))
}

function requiredPath(value, name) {
  if (value == null || value === true) {
    throw new Error(`${name} is required`)
  }
  return resolve(String(value))
}

function parseOptions(args) {
  const parsed = {}
  for (let index = 0; index < args.length; index += 1) {
    const argument = args[index]
    if (!argument.startsWith('--')) continue
    const key = argument.slice(2)
    const next = args[index + 1]
    if (next && !next.startsWith('--')) {
      parsed[key] = next
      index += 1
    } else {
      parsed[key] = true
    }
  }
  return parsed
}
