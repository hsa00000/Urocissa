import { spawn } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { existsSync, createWriteStream } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

import { compareSummaries } from './compare.mjs'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const backendDir = join(repoRoot, 'gallery-backend')
const frontendDir = join(repoRoot, 'gallery-frontend')
const backendBinary = resolve(
  process.env.UROCISSA_PERF_BACKEND ??
    join(
      backendDir,
      'target',
      'dev-release',
      process.platform === 'win32' ? 'urocissa.exe' : 'urocissa'
    )
)
const artifactRoot = join(repoRoot, '.performance')
const defaultSeed = 20260718n
const defaultCount = 100_000
const defaultSamples = 3
const viewport = { width: 1440, height: 900 }
const redbCacheBudgets = Object.freeze({
  mainBytes: 128 * 1024 ** 2,
  treeSnapshotBytes: 32 * 1024 ** 2,
  querySnapshotBytes: 16 * 1024 ** 2,
  expireBytes: 8 * 1024 ** 2,
  migrationSourceBytes: 128 * 1024 ** 2,
  migrationDestinationBytes: 128 * 1024 ** 2
})
const editMarkers = Object.freeze({
  albumTitle: 'Urocissa Performance Album',
  singleTag: 'urocissa-perf-single-edit',
  batchTag: 'urocissa-perf-batch-edit',
  description: 'Urocissa performance benchmark description',
  shareCreated: 'Urocissa performance share',
  shareUpdated: 'Urocissa performance share updated'
})
const editWorkload = Object.freeze({
  scope: 'metadata-and-state',
  batchSelection: 'all-visible-items',
  acknowledgement: 'ram-publish',
  durability: 'periodic-redb-write-behind',
  flushIntervalMs: 1000,
  softLimitMiB: 16,
  hardLimitMiB: 32,
  flushChunkRecords: 8192,
  snapshotSchema: 6,
  mediaObjects: 'stubbed-transparent-png',
  operations: [
    'album-create-title-cover',
    'share-create-update-delete',
    'description',
    'single-tags-albums-flags-trash',
    'batch-tags-albums-flags-trash',
    'chunk-failure-and-restart-partial-persistence'
  ]
})

const args = process.argv.slice(2)
const command = args.shift() ?? 'smoke'
const options = parseOptions(args)

if (!['baseline', 'compare', 'smoke', 'storage'].includes(command)) {
  console.error(`Unknown command: ${command}`)
  process.exit(2)
}

const count = Number(options.count ?? (command === 'smoke' ? 1_000 : command === 'storage' ? 1_000_000 : defaultCount))
const samples = Number(options.samples ?? (command === 'smoke' ? 1 : defaultSamples))
const seed = BigInt(options.seed ?? defaultSeed)
if (!Number.isSafeInteger(count) || count < 1 || count > 2_000_000) throw new Error('count must be an integer between 1 and 2,000,000')
if (!Number.isSafeInteger(samples) || samples < 1) throw new Error('samples must be a positive integer')
if (seed < 0n || seed > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('seed must be between 0 and Number.MAX_SAFE_INTEGER for JSON/u64 interoperability')
const headed = options.headed === true

async function main() {
  if (command === 'storage') {
    if (process.env.UROCISSA_PERF_SKIP_BUILD !== '1') await ensureBackendBuild()
    await runStorageSuite({ count, samples })
    return
  }

  if (!existsSync(join(repoRoot, 'performance', 'node_modules'))) {
    throw new Error('Performance dependencies are missing. Run: npm --prefix performance ci')
  }

  if (process.env.UROCISSA_PERF_SKIP_BUILD !== '1') await ensureBuilds()

  if (command === 'compare') {
    const baselinePath = resolve(options.baseline ?? join(artifactRoot, 'baseline', 'latest', 'summary.json'))
    const baseline = JSON.parse(await readFile(baselinePath, 'utf8'))
    const resultDir = join(artifactRoot, 'results', timestamp())
    const current = await runSuite({ resultDir, count, samples, seed, headed })
    const comparison = compareSummaries(baseline, current)
    const timingAdvisory = process.env.UROCISSA_PERF_ALLOW_TIMING_JITTER === '1'
    current.comparison = comparison
    await writeFile(join(resultDir, 'summary.json'), JSON.stringify(current, null, 2))
    await writeFile(join(resultDir, 'report.md'), renderReport(current, comparison))
    console.log(renderConsoleSummary(current, comparison))
    if (!current.correctness.ok || (!comparison.passed && !timingAdvisory)) {
      process.exitCode = 1
    }
    if (timingAdvisory && !comparison.passed) {
      console.log('timing jitter is advisory for this run')
    }
    return
  }

  const resultDir = command === 'baseline'
    ? join(artifactRoot, 'baseline', 'latest')
    : join(artifactRoot, 'smoke', timestamp())
  if (command === 'baseline') {
    await Promise.all([
      rm(join(resultDir, 'samples'), { recursive: true, force: true }),
      rm(join(resultDir, 'summary.json'), { force: true }),
      rm(join(resultDir, 'report.md'), { force: true })
    ])
  }
  const summary = await runSuite({ resultDir, count, samples, seed, headed })
  await writeFile(join(resultDir, 'summary.json'), JSON.stringify(summary, null, 2))
  await writeFile(join(resultDir, 'report.md'), renderReport(summary))
  console.log(renderConsoleSummary(summary))
  if (!summary.correctness.ok) process.exitCode = 1
}

async function ensureBuilds() {
  await ensureBackendBuild()
  if (!existsSync(join(frontendDir, 'node_modules'))) {
    await runCommand('npm', ['ci'], frontendDir)
  }
  await runCommand('npm', ['run', 'build:only'], frontendDir)
}

async function ensureBackendBuild() {
  await runCommand('cargo', ['build', '--profile', 'dev-release', '--features', 'performance-test'], backendDir)
}

async function runStorageSuite({ count, samples }) {
  const resultDir = join(artifactRoot, 'storage', timestamp())
  const root = await mkdtemp(join(tmpdir(), 'urocissa-storage-'))
  const marker = join(root, '.urocissa-performance-root')
  const resultPath = join(resultDir, 'storage-harness.json')
  const harnessLog = join(resultDir, 'storage-harness.log')
  const startupLog = join(resultDir, 'normal-startup.log')
  const startupEvents = join(resultDir, 'normal-startup.jsonl')
  let server = null

  await mkdir(resultDir, { recursive: true })
  await writeFile(marker, 'Disposable Urocissa storage benchmark data.\n')
  await mkdir(join(root, 'db'), { recursive: true })
  await mkdir(join(root, 'object', 'imported'), { recursive: true })
  await mkdir(join(root, 'object', 'compressed'), { recursive: true })
  await mkdir(join(root, 'upload'), { recursive: true })

  const port = await freePort()
  const token = randomBytes(24).toString('hex')
  const config = {
    public: {
      address: '127.0.0.1',
      port,
      limits: { json: '10MiB', file: '10GiB', 'data-form': '10GiB' },
      syncPaths: [],
      readOnlyMode: false,
      disableImg: false,
      writeBehind: { flushIntervalMs: 1000, softLimitMiB: 16, hardLimitMiB: 32 }
    },
    private: {
      password: 'urocissa-storage-benchmark',
      authKey: randomBytes(32).toString('hex'),
      discordHookUrl: null
    }
  }
  await writeFile(join(root, 'config.json'), JSON.stringify(config, null, 2))

  try {
    const harnessExitCode = await runStorageHarnessProcess({
      root,
      resultPath,
      logPath: harnessLog,
      count,
      samples
    })
    const report = JSON.parse(await readFile(resultPath, 'utf8'))

    const readinessStarted = Date.now()
    server = await startServer({ port, token, root, events: startupEvents, logPath: startupLog })
    const completeReadinessMs = Date.now() - readinessStarted
    const normalStartup = await perfFetch(port, token, '/__perf/status')
    await setPhase(port, token, 'storage-readiness-complete')
    await stopServer(server)
    server = null

    const maximumPeakRssBytes = Math.max(
      report.gate.maximumPeakRssBytes,
      normalStartup.backend_global_peak_rss_bytes
    )
    report.gate.maximumPeakRssBytes = maximumPeakRssBytes
    report.gate.rssPassed = maximumPeakRssBytes <= 850 * 1024 ** 2

    const summary = {
      ...report,
      generatedAt: new Date().toISOString(),
      source: sourceIdentity(),
      buildProfile: 'dev-release',
      completeReadinessMs,
      normalStartup,
      harnessExitCode
    }
    await writeFile(join(resultDir, 'summary.json'), JSON.stringify(summary, null, 2))
    await writeFile(join(resultDir, 'report.md'), renderStorageReport(summary))
    console.log(renderStorageConsoleSummary(summary))
    if (harnessExitCode !== 0 || !report.gate.relativeSpeedPassed || !report.gate.rssPassed) {
      process.exitCode = 1
    }
  } finally {
    await stopServer(server)
    await rm(root, { recursive: true, force: true })
  }
}

async function runStorageHarnessProcess({ root, resultPath, logPath, count, samples }) {
  const logStream = createWriteStream(logPath, { flags: 'a' })
  const child = spawn(backendBinary, [], {
    cwd: backendDir,
    env: {
      ...process.env,
      UROCISSA_PERF_ROOT: root,
      UROCISSA_STORAGE_HARNESS: '1',
      UROCISSA_STORAGE_COUNT: String(count),
      UROCISSA_STORAGE_SAMPLES: String(samples),
      UROCISSA_STORAGE_RESULT: resultPath
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true
  })
  child.stdout.pipe(logStream, { end: false })
  child.stderr.pipe(logStream, { end: false })
  const exitCode = await new Promise((resolvePromise, reject) => {
    child.once('error', reject)
    child.once('close', (code) => resolvePromise(code ?? 1))
  })
  logStream.end()
  return exitCode
}

function renderStorageConsoleSummary(summary) {
  return [
    `storage harness: ${summary.records.toLocaleString()} records, ${summary.samples} samples`,
    `migration: ${summary.migrationWallMs.toFixed(1)} ms (${Math.round(summary.migrationRecordsPerSecond).toLocaleString()} records/s)`,
    `V5 median startup scan/build: ${summary.gate.v5MedianMs.toFixed(1)} ms`,
    `V6 median startup scan/build: ${summary.gate.v6MedianMs.toFixed(1)} ms (${summary.gate.v6ToV5Ratio.toFixed(3)}x V5)`,
    `complete readiness: ${summary.completeReadinessMs.toFixed(1)} ms`,
    `peak RSS: ${(summary.gate.maximumPeakRssBytes / 1024 ** 2).toFixed(1)} MiB`,
    `gates: speed=${summary.gate.relativeSpeedPassed ? 'PASS' : 'FAIL'}, RSS=${summary.gate.rssPassed ? 'PASS' : 'FAIL'}`
  ].join('\n')
}

function renderStorageReport(summary) {
  return `# Urocissa V6 storage benchmark

- Records: ${summary.records.toLocaleString()}
- Samples: ${summary.samples}
- Migration: ${summary.migrationWallMs.toFixed(1)} ms (${Math.round(summary.migrationRecordsPerSecond).toLocaleString()} records/s)
- V5 median decode + TreeState: ${summary.gate.v5MedianMs.toFixed(1)} ms
- V6 median decode + TreeState: ${summary.gate.v6MedianMs.toFixed(1)} ms
- V6 / V5: ${summary.gate.v6ToV5Ratio.toFixed(3)} (limit 1.150)
- Complete readiness: ${summary.completeReadinessMs.toFixed(1)} ms
- Peak RSS: ${(summary.gate.maximumPeakRssBytes / 1024 ** 2).toFixed(1)} MiB (limit 850 MiB)
- Normal startup record iterations: ${summary.normalStartupRecordIterations}
- Migration record scans: source=${summary.migrationSourceRecordScans}, destination=${summary.migrationDestinationRecordScans}
- Gates: speed=${summary.gate.relativeSpeedPassed ? 'PASS' : 'FAIL'}, RSS=${summary.gate.rssPassed ? 'PASS' : 'FAIL'}

The JSON summary contains per-sample storage-open, O(1) count, decode scan, TreeState, throughput, RSS, and redb cache metrics.
`
}

async function runSuite({ resultDir, count, samples, seed, headed }) {
  await mkdir(join(resultDir, 'samples'), { recursive: true })
  const summary = {
    schemaVersion: 7,
    generatedAt: new Date().toISOString(),
    source: sourceIdentity(),
    environment: {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
      viewport,
      browser: 'chromium',
      buildProfile: 'dev-release',
      redbCacheBudgets,
      fixture: { count, samples, seed: seed.toString() },
      workload: editWorkload
    },
    samples: [],
    correctness: { ok: true, errors: [] }
  }

  for (let sampleIndex = 0; sampleIndex < samples; sampleIndex += 1) {
    console.log(`sample ${sampleIndex + 1}/${samples}: creating isolated database (${count} records)`)
    const sampleDir = join(resultDir, 'samples', String(sampleIndex + 1))
    await mkdir(sampleDir, { recursive: true })
    try {
      const sample = await runSample({ sampleDir, count, seed, sampleIndex, headed })
      summary.samples.push(sample)
      if (!sample.correctness.ok) {
        summary.correctness.ok = false
        summary.correctness.errors.push(...sample.correctness.errors)
      }
    } catch (error) {
      const message = error instanceof Error ? error.stack ?? error.message : String(error)
      summary.correctness.ok = false
      summary.correctness.errors.push(message)
      await writeFile(join(sampleDir, 'error.txt'), message)
      console.error(message)
    }
  }

  summary.aggregates = aggregateSamples(summary.samples)
  return summary
}

async function runSample({ sampleDir, count, seed, sampleIndex, headed }) {
  const root = await mkdtemp(join(tmpdir(), 'urocissa-perf-'))
  const marker = join(root, '.urocissa-performance-root')
  await writeFile(marker, 'This directory is disposable performance-test data.\n')
  await mkdir(join(root, 'db'), { recursive: true })
  await mkdir(join(root, 'object', 'imported'), { recursive: true })
  await mkdir(join(root, 'object', 'compressed'), { recursive: true })
  await mkdir(join(root, 'upload'), { recursive: true })

  const port = await freePort()
  const token = randomBytes(24).toString('hex')
  const config = {
    public: {
      address: '127.0.0.1',
      port,
      limits: { json: '10MiB', file: '10GiB', 'data-form': '10GiB' },
      syncPaths: [],
      readOnlyMode: false,
      disableImg: false,
      writeBehind: { flushIntervalMs: 1000, softLimitMiB: 16, hardLimitMiB: 32 }
    },
    private: { password: 'urocissa-performance-password', authKey: randomBytes(32).toString('hex'), discordHookUrl: null }
  }
  await writeFile(join(root, 'config.json'), JSON.stringify(config, null, 2))

  const firstEvents = join(sampleDir, 'backend-seed.jsonl')
  const firstLog = join(sampleDir, 'backend-seed.log')
  let first = null
  let second = null
  let third = null
  try {
    first = await startServer({ port, token, root, events: firstEvents, logPath: firstLog })
    await setPhase(port, token, 'fixture.insert')
    const seedStart = Date.now()
    const seedResponse = await perfFetch(port, token, '/__perf/fixture', {
      method: 'POST',
      body: JSON.stringify({ count, seed: Number(seed) }),
      headers: { 'content-type': 'application/json' }
    })
    const seedWallMs = Date.now() - seedStart
    await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
    const fixtureStatus = await perfFetch(port, token, '/__perf/status')
    await stopServer(first)
    first = null

    const secondEvents = join(sampleDir, 'backend-runtime.jsonl')
    const secondLog = join(sampleDir, 'backend-runtime.log')
    const startupStart = Date.now()
    second = await startServer({ port, token, root, events: secondEvents, logPath: secondLog })
    const startup = await waitForStatus(port, token, (value) => value.disk_count === count && value.memory_count === count)
    const startupWallMs = Date.now() - startupStart

    const browser = await runBrowserJourney({
      port,
      token,
      sampleDir,
      sampleIndex,
      headed,
      expectedHome: seedResponse.expected_home
    })

    await setPhase(port, token, 'flush-failure-retry')
    const retryMarker = `${editMarkers.batchTag}-retry-${sampleIndex}`
    const retryProbe = await perfFetch(port, token, '/__perf/restart-probe', {
      method: 'POST',
      body: JSON.stringify({
        markerTag: retryMarker,
        commitsBeforeFailure: 0,
        targetLimit: Math.min(count, editWorkload.flushChunkRecords)
      }),
      headers: { 'content-type': 'application/json' }
    })
    const retryFailedStatus = await waitForStatus(
      port,
      token,
      (value) => value.write_behind_flush_failure_count > retryProbe.failureCountBefore
    )
    const retryDrainStatus = await perfFetch(port, token, '/__perf/drain', { method: 'POST' })
    const retryAudit = await perfFetch(port, token, '/__perf/audit', {
      method: 'POST',
      body: JSON.stringify({ markerTag: retryMarker, view: 'disk' }),
      headers: { 'content-type': 'application/json' }
    })

    await setPhase(port, token, 'restart-partial-persistence')
    const restartMarker = `${editMarkers.batchTag}-restart-${sampleIndex}`
    const commitsBeforeFailure = count > editWorkload.flushChunkRecords ? 1 : 0
    const restartProbe = await perfFetch(port, token, '/__perf/restart-probe', {
      method: 'POST',
      body: JSON.stringify({ markerTag: restartMarker, commitsBeforeFailure }),
      headers: { 'content-type': 'application/json' }
    })
    const failedFlushStatus = await waitForStatus(
      port,
      token,
      (value) => value.write_behind_flush_failure_count > restartProbe.failureCountBefore
    )
    await crashServer(second)
    second = null

    const thirdEvents = join(sampleDir, 'backend-restart.jsonl')
    const thirdLog = join(sampleDir, 'backend-restart.log')
    third = await startServer({ port, token, root, events: thirdEvents, logPath: thirdLog })
    const restartStatus = await waitForStatus(
      port,
      token,
      (value) => value.disk_count === count + 1 && value.memory_count === count + 1
    )
    const restartAudit = await perfFetch(port, token, '/__perf/audit', {
      method: 'POST',
      body: JSON.stringify({ markerTag: restartMarker, view: 'disk' }),
      headers: { 'content-type': 'application/json' }
    })

    await setPhase(port, token, 'delete')
    const deleteStart = Date.now()
    const deleteSummary = await perfFetch(port, token, '/__perf/fixture', { method: 'DELETE' })
    const deleteWallMs = Date.now() - deleteStart
    const finalStatus = await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
    await stopServer(third)
    third = null
    await rm(root, { recursive: true, force: true })

    const events = [
      ...await readEvents(firstEvents),
      ...await readEvents(secondEvents),
      ...await readEvents(thirdEvents)
    ]
    const correctness = checkCorrectness({
      count,
      seedResponse,
      fixtureStatus,
      startup,
      browser,
      deleteSummary,
      finalStatus,
      rootExists: existsSync(root),
      backendEvents: events,
      restartProbe,
      retryProbe,
      retryFailedStatus,
      retryDrainStatus,
      retryAudit,
      failedFlushStatus,
      restartStatus,
      restartAudit
    })

    return {
      sampleIndex,
      seedWallMs,
      startupWallMs,
      fixture: seedResponse,
      fixtureStatus,
      startup,
      browser,
      delete: { ...deleteSummary, wallMs: deleteWallMs },
      finalStatus,
      restartProbe,
      retryProbe,
      retryFailedStatus,
      retryDrainStatus,
      retryAudit,
      failedFlushStatus,
      restartStatus,
      restartAudit,
      backendEvents: events,
      correctness
    }
  } finally {
    await stopServer(third)
    await stopServer(second)
    await stopServer(first)
    await rm(root, { recursive: true, force: true })
  }
}

async function runBrowserJourney({ port, token, sampleDir, sampleIndex, headed, expectedHome }) {
  const { chromium } = await import('playwright')
  const baseUrl = `http://localhost:${port}`
  const browser = await chromium.launch({ headless: !headed })
  const context = await browser.newContext({
    viewport
  })
  const transparentPng = Buffer.from(
    'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
    'base64'
  )
  await context.route('**/object/**', async (route) => {
    await route.fulfill({ status: 200, contentType: 'image/png', body: transparentPng })
  })
  const page = await context.newPage()
  const responses = []
  const errors = []
  let authComplete = false
  let currentPhase = 'startup'
  const requestStarts = new WeakMap()
  const inFlightApiRequests = new Set()
  let lastApiActivityAt = Date.now()
  page.on('request', (request) => {
    if (isApplicationApi(request.url())) {
      requestStarts.set(request, Date.now())
      inFlightApiRequests.add(request)
      lastApiActivityAt = Date.now()
    }
  })
  page.on('response', (response) => {
    if (isApplicationApi(response.url())) {
      const request = response.request()
      const started = requestStarts.get(request)
      inFlightApiRequests.delete(request)
      lastApiActivityAt = Date.now()
      responses.push({
        url: response.url(),
        status: response.status(),
        method: request.method(),
        durationMs: started == null ? null : Date.now() - started,
        phase: currentPhase
      })
    }
    if (response.status() >= 400 && (authComplete || response.status() !== 401)) errors.push(`HTTP ${response.status()} ${response.url()}`)
  })
  page.on('requestfailed', (request) => {
    if (isApplicationApi(request.url())) {
      inFlightApiRequests.delete(request)
      lastApiActivityAt = Date.now()
    }
    const errorText = request.failure()?.errorText ?? 'failed'
    if (errorText !== 'net::ERR_ABORTED') errors.push(`REQUEST ${request.url()} ${errorText}`)
  })
  page.on('pageerror', (error) => { if (!error.message.includes('status code 401')) errors.push(`PAGE ${error.message}`) })
  page.on('console', (message) => {
    const text = message.text()
    if (message.type() === 'error' && !text.includes('No Worker is Working') && (authComplete || !text.includes('401'))) errors.push(`CONSOLE ${text}`)
  })

  await context.addInitScript(() => {
    const state = {
      longTasks: [],
      paints: [],
      frameGaps: [],
      lastFrame: null,
      heapStartBytes: null
    }
    window.__urocissaPerf = state
    if (typeof PerformanceObserver !== 'undefined') {
      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) state.longTasks.push(entry.duration)
        }).observe({ type: 'longtask', buffered: true })
      } catch {}
      try {
        new PerformanceObserver((list) => {
          for (const entry of list.getEntries()) state.paints.push({ name: entry.name, startTime: entry.startTime })
        }).observe({ type: 'paint', buffered: true })
      } catch {}
    }
    const tick = (now) => {
      if (state.lastFrame !== null) state.frameGaps.push(now - state.lastFrame)
      state.lastFrame = now
      requestAnimationFrame(tick)
    }
    requestAnimationFrame(tick)
  })

  try {
  const phases = []
  const phase = async (name, action) => {
    await setPhase(port, token, name)
    currentPhase = name
    const beforeResponses = responses.length
    await resetBrowserMetrics(page)
    const start = Date.now()
    let payload
    try {
      payload = await action()
    } catch (error) {
      const detail = await page.evaluate(() => ({ url: location.href, body: document.body?.innerText?.slice(0, 500) ?? '' })).catch(() => null)
      const message = `${name}: ${error instanceof Error ? error.message : String(error)}${detail ? ` [url=${detail.url} body=${JSON.stringify(detail.body)}]` : ''}`
      errors.push(message)
      throw new Error(message, { cause: error })
    }
    await waitForApiQuiet({ inFlightApiRequests, lastActivity: () => lastApiActivityAt, quietMs: 350 })
    await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
    const backendStatus = await perfFetch(port, token, '/__perf/status')
    const browserMetrics = await readBrowserMetrics(page)
    phases.push({ name, wallMs: Date.now() - start, responseCount: responses.length - beforeResponses, browserMetrics, backendStatus, payload })
    return payload
  }
  const navigateToCollection = async (url) => {
    const prefetch = page.waitForResponse((response) => {
      return response.status() === 200 && new URL(response.url()).pathname === '/get/prefetch'
    })
    await page.goto(url, { waitUntil: 'domcontentloaded' })
    await prefetch
    await waitForApiQuiet({
      inFlightApiRequests,
      lastActivity: () => lastApiActivityAt,
      quietMs: 350
    })
  }

  await phase('login', async () => {
    await page.goto(`${baseUrl}/home`, { waitUntil: 'domcontentloaded' })
    await page.getByRole('textbox', { name: 'Password' }).fill('urocissa-performance-password')
    const prefetch = page.waitForResponse((response) => response.url().includes('/get/prefetch') && response.status() === 200)
    await page.getByRole('button', { name: 'Login' }).click()
    await page.waitForURL('**/home')
    const payload = await (await prefetch).json()
    authComplete = true
    return payload
  })

  const imageContainer = page.locator('#image-container')
  try {
    await imageContainer.waitFor({ state: 'visible', timeout: 30_000 })
  } catch (error) {
    const detail = await page.evaluate(() => ({ url: location.href, body: document.body?.innerText?.slice(0, 1000) ?? '' })).catch(() => null)
    throw new Error(`image container did not appear${detail ? ` [url=${detail.url} body=${JSON.stringify(detail.body)}]` : ''}`, { cause: error })
  }
  await phase('scroll-top', async () => {
    await imageContainer.hover()
    for (let index = 0; index < 5; index += 1) await page.mouse.wheel(0, 900)
  })

  await phase('jump-middle-scroll', async () => {
    const scrollbar = page.locator('#scroll-bar')
    const box = await scrollbar.boundingBox()
    if (!box) throw new Error('timeline scrollbar is not visible')
    await page.mouse.click(box.x + box.width / 2, box.y + box.height * 0.5)
    await imageContainer.hover()
    for (let index = 0; index < 4; index += 1) await page.mouse.wheel(0, 850)
    for (let index = 0; index < 2; index += 1) await page.mouse.wheel(0, -850)
  })

  await phase('jump-end-scroll', async () => {
    const scrollbar = page.locator('#scroll-bar')
    const box = await scrollbar.boundingBox()
    if (!box) throw new Error('timeline scrollbar is not visible')
    await page.mouse.click(box.x + box.width / 2, box.y + box.height * 0.98)
    await imageContainer.hover()
    for (let index = 0; index < 2; index += 1) await page.mouse.wheel(0, -900)
  })

  const audit = (request, view = 'logical') => perfFetch(port, token, '/__perf/audit', {
    method: 'POST',
    body: JSON.stringify({ ...request, view }),
    headers: { 'content-type': 'application/json' }
  })

  let albumId = null
  let shareId = null
  let itemId = null
  let initialItem = null

  await phase('edit-album-create', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    await page.locator('#image-container').waitFor({ state: 'visible' })
    const response = await performAndWaitForApiResponse(
      page,
      'POST',
      '/post/create_empty_album',
      () => page.getByRole('button', { name: 'Create album' }).click()
    )
    albumId = await response.text()
    await page.waitForURL(`**/albums/view/${albumId}/read`)
    return { albumId }
  })
  assertBenchmark(typeof albumId === 'string' && albumId.length > 0, 'album creation did not return an id')
  let albumAudit = await audit({ albumId })
  assertBenchmark(albumAudit.album?.id === albumId, 'created album is missing from audit')

  await phase('edit-album-title', async () => {
    const input = page
      .locator(
        '.album-title-field input, input[placeholder="Add Title"], input[placeholder="Untitled"]',
      )
      .first()
    await input.waitFor({ state: 'visible' })
    await performAndWaitForApiResponse(page, 'PUT', '/put/set_album_title', async () => {
      await input.fill(editMarkers.albumTitle)
      await input.press('Tab')
    })
  })
  albumAudit = await audit({ albumId })
  assertBenchmark(albumAudit.album?.title === editMarkers.albumTitle, 'album title was not persisted')

  await phase('edit-share-create', async () => {
    await page.getByRole('button', { name: 'Share album' }).click()
    const dialog = page.getByRole('dialog').filter({ hasText: 'Share Settings' })
    await dialog.getByLabel('Link Description').fill(editMarkers.shareCreated)
    const response = await performAndWaitForApiResponse(
      page,
      'POST',
      '/post/create_share',
      () => dialog.getByRole('button', { name: 'Create Link' }).click()
    )
    shareId = await response.text()
    await dialog.getByRole('button', { name: 'Copy' }).waitFor({ state: 'visible' })
    return { shareId }
  })
  assertBenchmark(typeof shareId === 'string' && shareId.length > 0, 'share creation did not return an id')
  let shareAudit = await audit({ albumId, shareId })
  assertBenchmark(shareAudit.album?.share?.description === editMarkers.shareCreated, 'created share is missing from audit')

  await phase('edit-share-update', async () => {
    const dialog = page.getByRole('dialog').filter({ hasText: 'Share Settings' })
    await dialog.getByLabel('Link Description').fill(editMarkers.shareUpdated)
    await performAndWaitForApiResponse(
      page,
      'PUT',
      '/put/edit_share',
      () => dialog.getByRole('button', { name: 'Save Changes' }).click()
    )
  })
  shareAudit = await audit({ albumId, shareId })
  assertBenchmark(shareAudit.album?.share?.description === editMarkers.shareUpdated, 'share update was not persisted')
  await page.getByRole('button', { name: 'Close dialog' }).click()

  await phase('edit-share-delete', async () => {
    await page.goto(`${baseUrl}/links`, { waitUntil: 'domcontentloaded' })
    await page.reload({ waitUntil: 'domcontentloaded' })
    const deleteButton = page.getByRole('button', { name: 'Delete share' })
    await deleteButton.waitFor({ state: 'visible' })
    await deleteButton.click()
    const dialog = page.locator('#delete-share-modal')
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/delete_share',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  shareAudit = await audit({ albumId, shareId })
  assertBenchmark(shareAudit.album?.shareCount === 0 && shareAudit.album.share == null, 'share deletion was not persisted')

  await phase('edit-open-single', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    await page.locator('#image-container').waitFor({ state: 'visible' })
    // The benchmark album is the newest Home item; choose the next item so
    // the single-item journey always targets fixture media.
    const firstItem = page.getByTestId('gallery-item').nth(1)
    await firstItem.waitFor({ state: 'visible' })
    await firstItem.getByTestId('open-item').dispatchEvent('click')
    await page.waitForURL('**/home/view/*')
    itemId = new URL(page.url()).pathname.split('/').at(-1)
    return { itemId }
  })
  assertBenchmark(typeof itemId === 'string' && itemId.length > 0, 'single-item navigation did not expose an id')
  initialItem = (await audit({ itemId })).item
  assertBenchmark(initialItem?.id === itemId, 'single-item audit did not find the selected media')

  await phase('edit-single-description', async () => {
    await page.getByRole('button', { name: 'Show info' }).click()
    const description = page
      .locator(
        '[data-testid="edit-description"] textarea:not([readonly]), textarea[data-testid="edit-description"]:not([readonly]), .v-textarea textarea:not([readonly])',
      )
      .first()
    await description.waitFor({ state: 'visible' })
    await performAndWaitForApiResponse(
      page,
      'PUT',
      '/put/set_user_defined_description',
      async () => {
        await description.fill(editMarkers.description)
        await description.press('Tab')
      }
    )
  })
  let itemAudit = await audit({ itemId })
  assertBenchmark(itemAudit.item?.description === editMarkers.description, 'description was not persisted')

  await phase('edit-single-tag-add', async () => {
    await openAction(page, 'Media actions', 'Edit Tags')
    const dialog = page.locator('#edit-tag-overlay')
    await addComboboxValue(dialog.getByLabel('Tags'), editMarkers.singleTag)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_tag',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  itemAudit = await audit({ itemId, markerTag: editMarkers.singleTag })
  assertBenchmark(itemAudit.item?.tags.includes(editMarkers.singleTag), 'single tag add was not persisted')

  await phase('edit-single-tag-remove', async () => {
    await openAction(page, 'Media actions', 'Edit Tags')
    const dialog = page.locator('#edit-tag-overlay')
    await removeChip(dialog, editMarkers.singleTag)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_tag',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  itemAudit = await audit({ itemId, markerTag: editMarkers.singleTag })
  assertBenchmark(!itemAudit.item?.tags.includes(editMarkers.singleTag) && itemAudit.marker.total === 0, 'single tag remove was not persisted')

  await phase('edit-single-album-add', async () => {
    await openAction(page, 'Media actions', 'Edit Albums')
    const dialog = page.locator('#edit-album-overlay')
    await selectComboboxOption(page, dialog.getByLabel('Albums'), editMarkers.albumTitle)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_album',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  itemAudit = await audit({ itemId, albumId })
  assertBenchmark(itemAudit.item?.albums.includes(albumId), 'single album add was not persisted')

  await phase('edit-single-album-remove', async () => {
    await page.reload({ waitUntil: 'domcontentloaded' })
    await page.getByRole('button', { name: 'Media actions' }).waitFor({ state: 'visible' })
    await openAction(page, 'Media actions', 'Edit Albums')
    const dialog = page.locator('#edit-album-overlay')
    await removeChip(dialog, editMarkers.albumTitle)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_album',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  itemAudit = await audit({ itemId, albumId })
  assertBenchmark(!itemAudit.item?.albums.includes(albumId), 'single album remove was not persisted')

  await phase('edit-single-flags', async () => {
    for (const name of ['Toggle favorite', 'Toggle favorite', 'Toggle archive', 'Toggle archive']) {
      await performAndWaitForApiResponse(
        page,
        'PUT',
        '/put/edit_flags',
        () => page.getByRole('button', { name }).click()
      )
    }
  })
  itemAudit = await audit({ itemId })
  assertBenchmark(
    itemAudit.item?.isFavorite === initialItem.isFavorite &&
      itemAudit.item?.isArchived === initialItem.isArchived,
    'single favorite/archive flags were not restored'
  )

  await phase('edit-single-trash-restore', async () => {
    await performActionAndWaitForApiResponse(page, 'Media actions', 'Delete', 'PUT', '/put/edit_flags')
    const deletedAudit = await audit({ itemId })
    assertBenchmark(deletedAudit.item?.isTrashed === true, 'single delete did not reach backend state')
    await navigateToCollection(`${baseUrl}/trashed/view/${itemId}`)
    await page.getByRole('button', { name: 'Media actions' }).waitFor({ state: 'visible' })
    await performActionAndWaitForApiResponse(page, 'Media actions', 'Restore', 'PUT', '/put/edit_flags')
  })
  itemAudit = await audit({ itemId })
  assertBenchmark(itemAudit.item?.isTrashed === initialItem.isTrashed, 'single trash flag was not restored')

  const expectedEditedHome = expectedHome + 1
  const selectedHome = await phase('edit-batch-select-all', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    return selectAllVisibleItems(page, expectedEditedHome)
  })
  assertBenchmark(
    selectedHome === expectedEditedHome,
    `home select-all chose ${selectedHome}; expected ${expectedEditedHome}`
  )

  await phase('edit-batch-tag-add', async () => {
    await openAction(page, 'Batch actions', 'Batch Edit Tags')
    const dialog = page.locator('#batch-edit-tag-overlay')
    await addComboboxValue(dialog.getByLabel('Add Tags'), editMarkers.batchTag)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_tag',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  let batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.total === selectedHome, `batch tag add reached ${batchAudit.marker.total}; expected ${selectedHome}`)

  await phase('edit-batch-favorite', async () => {
    await selectAllVisibleItems(page, expectedEditedHome)
    await performActionAndWaitForApiResponse(page, 'Batch actions', 'Favorite', 'PUT', '/put/edit_flags')
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.favorite === selectedHome, 'batch favorite did not update every marked item')

  await phase('edit-batch-flags-clear', async () => {
    await selectAllVisibleItems(page, expectedEditedHome)
    await openAction(page, 'Batch actions', 'Batch Edit Tags')
    const dialog = page.locator('#batch-edit-tag-overlay')
    const field = dialog.getByLabel('Remove Tags')
    await selectComboboxOption(page, field, 'Favorite')
    const selectedFlags = await field
      .locator('xpath=ancestor::*[contains(@class,"v-input")][1]')
      .locator('.v-chip')
      .allTextContents()
    assertBenchmark(
      selectedFlags.some((value) => value.includes('Favorite')),
      `batch flag clear selected unexpected chips: ${JSON.stringify(selectedFlags)}`
    )
    const response = await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_flags',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
    const payload = response.request().postDataJSON()
    assertBenchmark(
      payload.isFavorite === false && payload.isArchived === undefined,
      `batch flag clear sent unexpected payload: ${JSON.stringify(payload)}`
    )
    assertBenchmark(
      payload.selection?.mode === 'allExcept' && payload.selection.excludedIndices?.length === 0,
      `batch flag clear sent unexpected selection: ${JSON.stringify(payload.selection)}`
    )
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(
    batchAudit.marker.favorite === 0,
    `batch favorite was not cleared: favorite=${batchAudit.marker.favorite}`
  )

  await phase('edit-batch-album-add', async () => {
    await selectAllVisibleItems(page, expectedEditedHome)
    await openAction(page, 'Batch actions', 'Batch Edit Albums')
    const dialog = page.locator('#batch-edit-album-overlay')
    await selectComboboxOption(
      page,
      dialog.getByRole('combobox', { name: 'Add to Albums', exact: true }),
      editMarkers.albumTitle
    )
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_album',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  const expectedAlbumMembers = selectedHome - 1
  assertBenchmark(
    batchAudit.marker.albumMembers === expectedAlbumMembers &&
      batchAudit.album?.scannedMemberCount === expectedAlbumMembers &&
      batchAudit.album.itemCount === expectedAlbumMembers,
    `batch album add did not update membership and album metadata: ${JSON.stringify(batchAudit)}; selected=${selectedHome}`
  )

  await phase('edit-album-cover', async () => {
    await navigateToCollection(`${baseUrl}/albums/view/${albumId}/read`)
    await selectFirstItem(page)
    await performActionAndWaitForApiResponse(page, 'Batch actions', 'Set as Cover', 'PUT', '/put/set_album_cover')
  })
  albumAudit = await audit({ albumId })
  assertBenchmark(albumAudit.album?.cover != null, 'album cover was not persisted')

  await phase('edit-batch-album-remove', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    await selectAllVisibleItems(page, expectedEditedHome)
    await openAction(page, 'Batch actions', 'Batch Edit Albums')
    const dialog = page.locator('#batch-edit-album-overlay')
    await selectComboboxOption(
      page,
      dialog.getByRole('combobox', { name: 'Remove from Albums', exact: true }),
      editMarkers.albumTitle
    )
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_album',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(
    batchAudit.marker.albumMembers === 0 &&
      batchAudit.album?.scannedMemberCount === 0 &&
      batchAudit.album.itemCount === 0,
    'batch album remove left membership behind'
  )

  await phase('edit-batch-trash', async () => {
    await selectAllVisibleItems(page, expectedEditedHome)
    await performActionAndWaitForApiResponse(page, 'Batch actions', 'Delete', 'PUT', '/put/edit_flags')
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.trashed === selectedHome, 'batch trash did not update every marked item')

  await phase('edit-batch-restore', async () => {
    await navigateToCollection(`${baseUrl}/trashed`)
    await selectAllVisibleItems(page)
    await performActionAndWaitForApiResponse(page, 'Batch actions', 'Restore', 'PUT', '/put/edit_flags')
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.trashed === 0, 'batch restore left marked items trashed')

  await phase('edit-batch-archive', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    await selectAllVisibleItems(page)
    await performActionAndWaitForApiResponse(page, 'Batch actions', 'Archive', 'PUT', '/put/edit_flags')
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.archived === selectedHome, 'batch archive did not update every marked item')

  await phase('edit-batch-archive-clear', async () => {
    await navigateToCollection(`${baseUrl}/archived`)
    await selectAllVisibleItems(page)
    await openAction(page, 'Batch actions', 'Batch Edit Tags')
    const dialog = page.locator('#batch-edit-tag-overlay')
    const field = dialog.getByLabel('Remove Tags')
    await selectComboboxOption(page, field, 'Archived')
    const response = await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_flags',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
    const payload = response.request().postDataJSON()
    assertBenchmark(
      payload.isArchived === false,
      `batch archive clear sent unexpected payload: ${JSON.stringify(payload)}`
    )
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(
    batchAudit.marker.archived === 0,
    `batch archive was not cleared: archived=${batchAudit.marker.archived}`
  )

  await phase('edit-batch-tag-remove', async () => {
    await navigateToCollection(`${baseUrl}/home`)
    await selectAllVisibleItems(page)
    await openAction(page, 'Batch actions', 'Batch Edit Tags')
    const dialog = page.locator('#batch-edit-tag-overlay')
    await addComboboxValue(dialog.getByLabel('Remove Tags'), editMarkers.batchTag)
    await performDialogAndWaitForApiResponse(
      page,
      dialog,
      'PUT',
      '/put/edit_tag',
      () => dialog.getByRole('button', { name: 'OK', exact: true }).click()
    )
  })
  batchAudit = await audit({ albumId, markerTag: editMarkers.batchTag })
  assertBenchmark(batchAudit.marker.total === 0, 'batch tag remove left marker tags behind')

  const drainStatus = await phase('write-behind-drain', async () => {
    return perfFetch(port, token, '/__perf/drain', { method: 'POST' })
  })
  assertBenchmark(
    drainStatus.write_behind_pending_operations === 0 && drainStatus.write_behind_pending_bytes === 0,
    `write-behind drain left pending data: ${JSON.stringify(drainStatus)}`
  )

  const diskAfterDrain = await audit(
    { itemId, albumId, markerTag: editMarkers.batchTag, shareId },
    'disk'
  )
  assertBenchmark(
    JSON.stringify(diskAfterDrain) === JSON.stringify(await audit(
      { itemId, albumId, markerTag: editMarkers.batchTag, shareId },
      'logical'
    )),
    'logical and disk audit diverged after drain'
  )

  await phase('warm-reload', async () => {
    const prefetch = page.waitForResponse((response) => response.url().includes('/get/prefetch') && response.status() === 200)
    await page.reload({ waitUntil: 'domcontentloaded' })
    return await (await prefetch).json()
  })

  const finalAudit = await audit(
    { itemId, albumId, markerTag: editMarkers.batchTag, shareId },
    'disk'
  )
  assertBenchmark(finalAudit.diskCount > 0, 'final audit did not read the edited fixture')
  assertBenchmark(finalAudit.item?.description === editMarkers.description, 'description was lost after reload')
  assertBenchmark(
    !finalAudit.item?.tags.includes(editMarkers.singleTag) &&
      !finalAudit.item?.tags.includes(editMarkers.batchTag) &&
      !finalAudit.item?.albums.includes(albumId),
    'single tag or album membership returned after reload'
  )
  assertBenchmark(
    finalAudit.item?.isFavorite === initialItem.isFavorite &&
      finalAudit.item?.isArchived === initialItem.isArchived &&
      finalAudit.item?.isTrashed === initialItem.isTrashed,
    'single flags were not restored after reload'
  )
  assertBenchmark(finalAudit.album?.title === editMarkers.albumTitle, 'album title was lost after reload')
  assertBenchmark(finalAudit.album?.shareCount === 0, 'deleted share returned after reload')
  assertBenchmark(finalAudit.album?.scannedMemberCount === 0, 'album membership returned after reload')
  assertBenchmark(finalAudit.marker.total === 0, 'batch marker returned after reload')

  const navigation = await page.evaluate(() => {
    const navigationEntry = performance.getEntriesByType('navigation')[0]
    return navigationEntry ? {
      domContentLoaded: navigationEntry.domContentLoadedEventEnd,
      load: navigationEntry.loadEventEnd,
      response: navigationEntry.responseEnd
    } : null
  })
  await page.screenshot({ path: join(sampleDir, `browser-${sampleIndex}.png`), fullPage: false })
  return { phases, navigation, responses, errors, finalAudit }
  } finally {
    await context.close().catch(() => {})
    await browser.close().catch(() => {})
  }
}

async function performAndWaitForApiResponse(page, method, path, action) {
  const [response] = await Promise.all([
    page.waitForResponse((candidate) => {
      const request = candidate.request()
      return request.method() === method && new URL(candidate.url()).pathname === path
    }),
    action()
  ])
  if (!response.ok()) throw new Error(`${method} ${path} returned ${response.status()}`)
  return response
}

async function performDialogAndWaitForApiResponse(page, dialog, method, path, action) {
  const response = await performAndWaitForApiResponse(page, method, path, action)
  await dialog.waitFor({ state: 'hidden' })
  return response
}

async function performActionAndWaitForApiResponse(page, menuName, actionName, method, path) {
  try {
    return await performAndWaitForApiResponse(
      page,
      method,
      path,
      () => openAction(page, menuName, actionName)
    )
  } catch (error) {
    const diagnostics = await page
      .evaluate(() => ({
        route: location.pathname,
        snackbars: Array.from(document.querySelectorAll('.v-snackbar'))
          .map((element) => element.textContent?.trim() ?? '')
          .filter(Boolean),
        activeOverlays: Array.from(document.querySelectorAll('.v-overlay--active'))
          .map((element) => element.textContent?.trim() ?? '')
          .filter(Boolean)
      }))
      .catch(() => null)
    throw new Error(
      `${menuName}/${actionName}: ${error instanceof Error ? error.message : String(error)}` +
        (diagnostics ? ` diagnostics=${JSON.stringify(diagnostics)}` : ''),
      { cause: error }
    )
  }
}

async function openAction(page, menuName, actionName) {
  await page.getByRole('button', { name: menuName }).click()
  const actionLabel = page
    .locator('.v-overlay--active .v-list-item')
    .getByText(actionName, { exact: true })
    .last()
  await actionLabel.waitFor({ state: 'visible' })
  const action = actionLabel.locator(
    'xpath=ancestor::*[contains(concat(" ", normalize-space(@class), " "), " v-list-item ")][1]'
  )
  await action.click()
}

async function addComboboxValue(field, value) {
  await field.click()
  await field.fill(value)
  await field.press('Enter')
  const input = field.locator('xpath=ancestor::*[contains(@class,"v-input")][1]')
  await input.getByText(value, { exact: true }).last().waitFor({ state: 'visible' })
}

async function selectComboboxOption(page, field, value) {
  const input = field.locator('xpath=ancestor::*[contains(@class,"v-input")][1]')
  await input.locator('.v-field__input').click()
  await field.press('ArrowDown')
  const option = page
    .locator('.v-overlay--active .v-list-item')
    .filter({ has: page.getByText(value, { exact: true }) })
    .last()
  await option.waitFor({ state: 'visible' })
  await option.click()
  await input.locator('.v-chip').filter({ hasText: value }).last().waitFor({ state: 'visible' })
  await field.evaluate((element) => {
    const boundary = element.closest('.v-overlay__content')
    if (!boundary) throw new Error('combobox dialog boundary is missing')
    boundary.addEventListener(
      'keydown',
      (event) => {
        if (event.key === 'Escape') event.stopPropagation()
      },
      { once: true }
    )
  })
  await field.press('Escape')
  await option.waitFor({ state: 'hidden' })
}

async function removeChip(dialog, value) {
  const chip = dialog.locator('.v-chip').filter({ hasText: value }).last()
  await chip.waitFor({ state: 'visible' })
  await chip.locator('.v-chip__close').click()
}

async function selectFirstItem(page) {
  const imageContainer = page.locator('#image-container').last()
  await imageContainer.waitFor({ state: 'visible' })
  const firstItem = imageContainer.getByTestId('gallery-item').first()
  await firstItem.waitFor({ state: 'visible' })
  await firstItem.getByTestId('select-item').dispatchEvent('click')
  await page.getByText('1 items', { exact: true }).waitFor({ state: 'visible' })
}

async function selectAllVisibleItems(page, expected = null) {
  const clearSelection = page.getByRole('button', { name: 'Clear selection' })
  if (await clearSelection.count()) await clearSelection.click()
  await selectFirstItem(page)
  const selectAll = page.getByRole('button', { name: 'Select all' })
  if (await selectAll.count()) await selectAll.click()
  const selectedText = page.getByText(/^\d+ items$/, { exact: true }).last()
  await selectedText.waitFor({ state: 'visible' })
  const selected = Number.parseInt((await selectedText.innerText()).split(' ', 1)[0], 10)
  if (expected != null) {
    assertBenchmark(selected === expected, `select-all chose ${selected}; expected ${expected}`)
  }
  return selected
}

function assertBenchmark(condition, message) {
  if (!condition) throw new Error(`benchmark correctness: ${message}`)
}

function isApplicationApi(url) {
  const path = new URL(url).pathname
  return /^\/(?:get|post|put|delete)\//.test(path) || path === '/upload'
}

async function resetBrowserMetrics(page) {
  await page.evaluate(() => {
    const state = window.__urocissaPerf
    if (!state) return
    state.longTasks.length = 0
    state.paints.length = 0
    state.frameGaps.length = 0
    state.lastFrame = null
    state.heapStartBytes = performance.memory?.usedJSHeapSize ?? null
  })
}

async function readBrowserMetrics(page) {
  return page.evaluate(() => {
    const state = window.__urocissaPerf ?? {
      longTasks: [],
      paints: [],
      frameGaps: [],
      heapStartBytes: null
    }
    const sortedGaps = [...state.frameGaps].sort((a, b) => a - b)
    const percentile = (values, p) => values.length ? values[Math.min(values.length - 1, Math.floor(values.length * p))] : 0
    const heapUsedBytes = performance.memory?.usedJSHeapSize ?? null
    return {
      longTaskCount: state.longTasks.length,
      longTaskTotalMs: state.longTasks.reduce((sum, value) => sum + value, 0),
      longTaskMaxMs: Math.max(0, ...state.longTasks),
      paintCount: state.paints.length,
      paintEntries: state.paints,
      frameCount: state.frameGaps.length,
      frameGapP95Ms: percentile(sortedGaps, 0.95),
      frameGapMaxMs: Math.max(0, ...state.frameGaps),
      heapUsedBytes,
      heapDeltaBytes: heapUsedBytes == null || state.heapStartBytes == null
        ? null
        : heapUsedBytes - state.heapStartBytes
    }
  })
}

async function readEvents(path) {
  if (!existsSync(path)) return []
  const text = await readFile(path, 'utf8')
  const events = text.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line))
  const latestStart = events.findLastIndex((event) => event.sequence === 0)
  return latestStart === -1 ? events : events.slice(latestStart)
}

function aggregateSamples(samples) {
  const values = new Map()
  for (const sample of samples) {
    for (const phase of sample.browser?.phases ?? []) {
      addValue(values, `browser.${phase.name}.wallMs`, phase.wallMs)
      for (const [metric, value] of Object.entries(phase.browserMetrics ?? {})) {
        if (typeof value === 'number') {
          addValue(values, `browser.phase.${phase.name}.metrics.${metric}`, value)
        }
      }
      addStatusMemory(values, `backend.phase.${phase.name}`, phase.backendStatus)
      addValue(values, `backend.phase.${phase.name}.dirtyBytes`, phase.backendStatus?.write_behind_pending_bytes)
      addValue(values, `backend.phase.${phase.name}.pendingRecords`, phase.backendStatus?.write_behind_pending_records)
      addValue(values, `backend.phase.${phase.name}.activeRecords`, phase.backendStatus?.write_behind_active_records)
      addValue(values, `backend.phase.${phase.name}.flushingRecords`, phase.backendStatus?.write_behind_flushing_records)
      addValue(values, `backend.phase.${phase.name}.estimatedDrainMs`, phase.backendStatus?.write_behind_estimated_drain_ms)
      addValue(values, `backend.phase.${phase.name}.flushRecordsPerSecond`, phase.backendStatus?.write_behind_flush_records_per_second)
      addValue(values, `backend.phase.${phase.name}.lastFlushRecords`, phase.backendStatus?.write_behind_last_flush_records)
      addValue(values, `backend.phase.${phase.name}.lastFlushUniqueRecords`, phase.backendStatus?.write_behind_last_flush_unique_records)
      addValue(values, `backend.phase.${phase.name}.lastFlushChunks`, phase.backendStatus?.write_behind_last_flush_chunks)
    }
    for (const response of sample.browser?.responses ?? []) {
      if (response.durationMs == null) continue
      const route = new URL(response.url).pathname.replaceAll('/', '_').replace(/^_/, '')
      addValue(values, `browser.network.${response.method.toLowerCase()}.${route}.ms`, response.durationMs)
      if (response.phase) {
        addValue(values, `browser.phase.${response.phase}.network.${response.method.toLowerCase()}.${route}.ms`, response.durationMs)
      }
    }
    addValue(values, 'server.fixture.totalMs', sample.fixture?.total_ns / 1e6)
    addValue(values, 'server.fixture.insertMs', sample.fixture?.insert_ns / 1e6)
    addValue(values, 'server.fixture.rebuildMs', sample.fixture?.rebuild_ns / 1e6)
    addStatusMemory(values, 'backend.stage.fixture', sample.fixtureStatus)
    addValue(values, 'backend.stage.fixture.flushRecordsPerSecond', sample.fixtureStatus?.write_behind_flush_records_per_second)
    addValue(values, 'server.startup.wallMs', sample.startupWallMs)
    addStatusMemory(values, 'backend.stage.startup', sample.startup)
    addStatusMemory(values, 'backend.stage.failedFlush', sample.failedFlushStatus)
    addValue(values, 'backend.stage.failedFlush.pendingRecords', sample.failedFlushStatus?.write_behind_pending_records)
    addValue(values, 'backend.stage.failedFlush.estimatedDrainMs', sample.failedFlushStatus?.write_behind_estimated_drain_ms)
    addStatusMemory(values, 'backend.stage.retry', sample.retryDrainStatus)
    addValue(values, 'backend.stage.retry.flushRecordsPerSecond', sample.retryDrainStatus?.write_behind_flush_records_per_second)
    addStatusMemory(values, 'backend.stage.restart', sample.restartStatus)
    addStatusMemory(values, 'backend.stage.cleanup', sample.finalStatus)
    addValue(values, 'server.delete.totalMs', sample.delete?.total_ns / 1e6)
    for (const event of sample.backendEvents ?? []) {
      if (event.operation && event.duration_ns != null) {
        addValue(values, `backend.${event.operation}.ms`, event.duration_ns / 1e6)
        if (event.phase) addValue(values, `backend.phase.${event.phase}.${event.operation}.ms`, event.duration_ns / 1e6)
      }
    }
  }
  return Object.fromEntries([...values].map(([key, list]) => [key, stats(list)]))
}

function addStatusMemory(values, prefix, status) {
  addValue(values, `${prefix}.rssBytes`, status?.backend_rss_bytes)
  addValue(values, `${prefix}.globalPeakRssBytes`, status?.backend_global_peak_rss_bytes)
  addValue(values, `${prefix}.phasePeakRssBytes`, status?.backend_phase_peak_rss_bytes)
  addValue(values, `${prefix}.phaseAverageRssBytes`, status?.backend_phase_average_rss_bytes)
  addValue(values, `${prefix}.phaseRssSampleCount`, status?.backend_phase_rss_sample_count)
  for (const [name, cache] of [
    ['main', status?.redb_main_cache],
    ['treeSnapshot', status?.redb_tree_snapshot_cache],
    ['querySnapshot', status?.redb_query_snapshot_cache],
    ['expire', status?.redb_expire_cache]
  ]) {
    addValue(values, `${prefix}.redbCache.${name}.limitBytes`, cache?.limit_bytes)
    addValue(values, `${prefix}.redbCache.${name}.usedBytes`, cache?.used_bytes)
    addValue(values, `${prefix}.redbCache.${name}.evictionCount`, cache?.evictions)
    addValue(values, `${prefix}.redbCache.${name}.readHitCount`, cache?.read_hits)
    addValue(values, `${prefix}.redbCache.${name}.readMissCount`, cache?.read_misses)
    addValue(values, `${prefix}.redbCache.${name}.writeHitCount`, cache?.write_hits)
    addValue(values, `${prefix}.redbCache.${name}.writeMissCount`, cache?.write_misses)
  }
  for (const [name, value] of Object.entries(status?.tree_memory ?? {})) {
    addValue(values, `${prefix}.component.tree.${snakeToCamel(name)}`, value)
  }
  addValue(values, `${prefix}.component.treeSnapshotBytes`, status?.tree_snapshot_memory_bytes)
  addValue(values, `${prefix}.component.querySnapshotBytes`, status?.query_snapshot_memory_bytes)
  addValue(values, `${prefix}.component.writeBehindBytes`, status?.write_behind_memory_bytes)
}

function snakeToCamel(value) {
  return value.replace(/_([a-z])/g, (_, letter) => letter.toUpperCase())
}

function renderReport(summary, comparison = null) {
  const lines = [`# Urocissa performance report`, '', `- Generated: ${summary.generatedAt}`, `- Samples: ${summary.samples.length}`, `- Fixture: ${summary.environment.fixture.count} records`, `- Correctness: ${summary.correctness.ok ? 'PASS' : 'FAIL'}`, '']
  if (summary.correctness.errors.length) lines.push('## Correctness errors', '', ...summary.correctness.errors.map((error) => `- ${error}`), '')
  lines.push('## Aggregate metrics', '', '| Metric | Median | P95 | Max |', '|---|---:|---:|---:|')
  for (const [key, value] of Object.entries(summary.aggregates ?? {})) {
    lines.push(
      `| ${key} | ${formatMetric(key, value.median)} | ${formatMetric(key, value.p95)} | ${formatMetric(key, value.max)} |`
    )
  }
  if (comparison) {
    lines.push('', '## Baseline comparison', '', '| Metric | Baseline | Current | Delta |', '|---|---:|---:|---:|')
    for (const [key, value] of Object.entries(comparison.metrics)) {
      lines.push(
        `| ${key} | ${formatMetric(key, value.baseline)} | ${formatMetric(key, value.current)} | ${formatMetricDelta(key, value.delta, value.relative)} |`
      )
    }
    lines.push(
      '',
      `Noise-adjusted zero-regression gate: ${comparison.passed ? 'PASS' : 'FAIL'}`,
      `Timing/throughput regressions: ${comparison.notableRegressions.length || 'none'}`
    )
  }
  return `${lines.join('\n')}\n`
}

function renderConsoleSummary(summary, comparison = null) {
  const lines = [`correctness: ${summary.correctness.ok ? 'PASS' : 'FAIL'}`, `samples: ${summary.samples.length}`, `fixture: ${summary.environment.fixture.count}`]
  if (comparison) {
    lines.push(`zero-regression gate: ${comparison.passed ? 'PASS' : 'FAIL'}`)
    lines.push(`timing/throughput regressions: ${comparison.notableRegressions.length || 'none'}`)
  }
  return lines.join('\n')
}

function checkCorrectness({
  count,
  seedResponse,
  fixtureStatus,
  startup,
  browser,
  deleteSummary,
  finalStatus,
  rootExists,
  backendEvents,
  restartProbe,
  retryProbe,
  retryFailedStatus,
  retryDrainStatus,
  retryAudit,
  failedFlushStatus,
  restartStatus,
  restartAudit
}) {
  const errors = []
  if (seedResponse?.inserted !== count) errors.push(`inserted ${seedResponse?.inserted ?? 'unknown'} of ${count}`)
  if (startup?.disk_count !== count || startup?.memory_count !== count) errors.push('restart readiness count mismatch')
  if (!browser || browser.errors?.length) errors.push(...(browser?.errors ?? ['browser journey did not complete']))
  if (browser?.finalAudit?.diskCount !== count + 1) {
    errors.push(`post-edit disk count mismatch: expected ${count + 1} got ${browser?.finalAudit?.diskCount ?? 'unknown'}`)
  }
  const phaseStatuses = browser?.phases?.map((phase) => phase.backendStatus) ?? []
  const backendStatuses = [
    fixtureStatus,
    startup,
    ...phaseStatuses,
    retryFailedStatus,
    retryDrainStatus,
    failedFlushStatus,
    restartStatus,
    finalStatus
  ]
  if (backendStatuses.some((status) => status?.write_behind_pending_bytes > 32 * 1024 * 1024)) {
    errors.push('write-behind dirty bytes exceeded 32 MiB')
  }
  if (
    count >= 1_000_000 &&
    backendStatuses.some((status) => status?.backend_global_peak_rss_bytes > 1.0 * 1024 ** 3)
  ) {
    errors.push('backend sampled process peak RSS exceeded the 1.0 GiB acceptance limit')
  }
  if (count >= 1_000_000 && startup?.backend_rss_bytes > 850 * 1024 ** 2) {
    errors.push('backend startup RSS exceeded the 850 MiB target')
  }
  const cacheChecks = [
    ['main', 'redb_main_cache', redbCacheBudgets.mainBytes],
    ['tree snapshot', 'redb_tree_snapshot_cache', redbCacheBudgets.treeSnapshotBytes],
    ['query snapshot', 'redb_query_snapshot_cache', redbCacheBudgets.querySnapshotBytes],
    ['expire', 'redb_expire_cache', redbCacheBudgets.expireBytes]
  ]
  for (const [name, field, expectedLimit] of cacheChecks) {
    if (backendStatuses.some((status) => status?.[field]?.limit_bytes !== expectedLimit)) {
      errors.push(`${name} Redb cache limit diverged from the schema 7 workload`)
    }
    if (backendStatuses.some((status) => status?.[field]?.used_bytes > expectedLimit)) {
      errors.push(`${name} Redb cache used bytes exceeded its fixed limit`)
    }
  }
  if (
    redbCacheBudgets.treeSnapshotBytes +
      redbCacheBudgets.querySnapshotBytes +
      redbCacheBudgets.expireBytes !==
    56 * 1024 ** 2
  ) {
    errors.push('derived Redb cache budgets do not total 56 MiB')
  }
  if (
    backendStatuses.some(
      (status) => status?.write_behind_flush_chunk_records !== editWorkload.flushChunkRecords
    ) || restartProbe?.flushChunkRecords !== editWorkload.flushChunkRecords
  ) {
    errors.push('benchmark workload flush chunk size diverged from backend runtime')
  }
  if ((backendEvents ?? []).some((event) => event.operation === 'tree.rebuild' && event.phase?.startsWith('edit-'))) {
    errors.push('metadata edit phase triggered a full tree rebuild')
  }
  if (!failedFlushStatus?.write_behind_flush_failure_count) {
    errors.push('write-behind chunk failure injection was not observed')
  }
  if (
    !retryFailedStatus?.write_behind_last_error ||
    retryDrainStatus?.write_behind_pending_operations !== 0 ||
    retryDrainStatus?.write_behind_flush_retry_count <= retryProbe?.retryCountBefore ||
    retryAudit?.marker?.total !== retryProbe?.targets
  ) {
    errors.push('write-behind did not retry and fully materialize the bounded failure probe')
  }
  if (!failedFlushStatus?.write_behind_last_error || !failedFlushStatus?.write_behind_pending_operations) {
    errors.push('failed flush did not retain pending operations before the crash')
  }
  if (restartStatus?.disk_count !== count + 1 || restartStatus?.memory_count !== count + 1) {
    errors.push('restart after the injected crash did not rebuild the complete durable tree')
  }
  const durableMarkerCount = restartAudit?.marker?.total
  if (durableMarkerCount !== restartProbe?.expectedDurableMin || durableMarkerCount !== restartProbe?.expectedDurableMax) {
    errors.push(
      `restart partial persistence mismatch: expected ${restartProbe?.expectedDurableMin ?? 'unknown'}, got ${durableMarkerCount ?? 'unknown'}`
    )
  }
  if (count > editWorkload.flushChunkRecords && !(durableMarkerCount > 0 && durableMarkerCount < restartProbe?.targets)) {
    errors.push('large-fixture restart probe did not preserve a strict partial chunk prefix')
  }
  if (deleteSummary?.found !== count + 1) {
    errors.push(`fixture deletion found ${deleteSummary?.found ?? 'unknown'} records; expected ${count + 1}`)
  }
  if (deleteSummary?.remaining !== 0 || finalStatus?.disk_count !== 0 || finalStatus?.memory_count !== 0) errors.push('fixture deletion left rows behind')
  if (rootExists) errors.push('temporary benchmark root was not removed')
  if (seedResponse?.seed?.toString() !== seed.toString()) errors.push(`seed mismatch: expected ${seed} got ${seedResponse?.seed}`)
  return { ok: errors.length === 0, errors }
}

function addValue(map, key, value) {
  if (!Number.isFinite(value)) return
  if (!map.has(key)) map.set(key, [])
  map.get(key).push(value)
}

function stats(values) {
  const sorted = [...values].sort((a, b) => a - b)
  return { count: sorted.length, min: sorted[0], median: percentile(sorted, 0.5), p95: percentile(sorted, 0.95), max: sorted.at(-1) }
}

function percentile(values, p) {
  return values[Math.min(values.length - 1, Math.floor(values.length * p))]
}

function formatMetric(key, value) {
  if (value == null) return '-'
  if (key.endsWith('Bytes')) return `${Math.round(value).toLocaleString('en-US')} B`
  if (key.endsWith('Count')) return Number.isInteger(value) ? String(value) : value.toFixed(3)
  return `${value.toFixed(3)} ms`
}

function formatMetricDelta(key, delta, relative) {
  if (delta == null) return '-'
  const sign = delta >= 0 ? '+' : ''
  return `${sign}${formatMetric(key, delta)} (${(relative * 100).toFixed(1)}%)`
}
function timestamp() { return new Date().toISOString().replaceAll(':', '').replaceAll('.', '') }
function sourceIdentity() { return { sha: runCommandSync('git', ['rev-parse', 'HEAD'], repoRoot), dirty: runCommandSync('git', ['status', '--porcelain'], repoRoot) !== '' } }

async function startServer({ port, token, root, events, logPath }) {
  const logStream = createWriteStream(logPath, { flags: 'a' })
  const child = spawn(backendBinary, [], {
    cwd: backendDir,
    env: { ...process.env, UROCISSA_PERF_ROOT: root, UROCISSA_PERF_TOKEN: token, UROCISSA_PERF_EVENTS: events },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true
  })
  child.stdout.pipe(logStream, { end: false })
  child.stderr.pipe(logStream, { end: false })
  child.on('error', (error) => logStream.write(`\nprocess error: ${error.stack ?? error}\n`))
  try {
    await waitForStatus(port, token, (value) => value.disk_count >= 0 && value.memory_count >= 0)
  } catch (error) {
    await stopServer({ child, logStream })
    throw error
  }
  return { child, logStream }
}

async function stopServer(server) {
  if (!server) return
  if (server.child.exitCode === null) {
    const closed = new Promise((resolveClose) => server.child.once('close', resolveClose))
    server.child.kill()
    const exited = await Promise.race([closed.then(() => true), sleep(2_000).then(() => false)])
    if (!exited && process.platform === 'win32') {
      try { execFileSync('taskkill', ['/pid', String(server.child.pid), '/t', '/f'], { stdio: 'ignore', windowsHide: true }) } catch {}
    } else if (!exited) {
      try { server.child.kill('SIGKILL') } catch {}
    }
  }
  server.logStream.end()
}

async function crashServer(server) {
  if (!server) return
  if (server.child.exitCode === null) {
    const closed = new Promise((resolveClose) => server.child.once('close', resolveClose))
    if (process.platform === 'win32') {
      try {
        execFileSync('taskkill', ['/pid', String(server.child.pid), '/t', '/f'], {
          stdio: 'ignore',
          windowsHide: true
        })
      } catch {
        if (server.child.exitCode === null) throw new Error('failed to terminate benchmark server')
      }
    } else {
      server.child.kill('SIGKILL')
    }
    await Promise.race([closed, sleep(5_000)])
  }
  server.logStream.end()
}

async function waitForStatus(port, token, predicate) {
  const deadline = Date.now() + 60_000
  let lastError = null
  while (Date.now() < deadline) {
    try {
      const status = await perfFetch(port, token, '/__perf/status')
      if (predicate(status)) return status
    } catch (error) { lastError = error }
    await sleep(100)
  }
  throw new Error(`server readiness timed out: ${lastError?.message ?? 'unknown error'}`)
}

async function perfFetch(port, token, path, init = {}) {
  const response = await fetch(`http://127.0.0.1:${port}${path}`, {
    ...init,
    headers: { 'x-urocissa-perf-token': token, ...(init.headers ?? {}) }
  })
  const text = await response.text()
  if (!response.ok) throw new Error(`${init.method ?? 'GET'} ${path} returned ${response.status}: ${text}`)
  return text ? JSON.parse(text) : null
}

async function setPhase(port, token, name) {
  await perfFetch(port, token, '/__perf/phase', { method: 'POST', body: JSON.stringify({ name }), headers: { 'content-type': 'application/json' } })
}

async function waitForApiQuiet({ inFlightApiRequests, lastActivity, quietMs }) {
  const deadline = Date.now() + 30_000
  while (Date.now() < deadline) {
    if (inFlightApiRequests.size === 0 && Date.now() - lastActivity() >= quietMs) return
    await sleep(50)
  }
  throw new Error(`application API did not become quiet; ${inFlightApiRequests.size} request(s) still active`)
}

async function freePort() {
  const { createServer } = await import('node:net')
  const server = createServer()
  await new Promise((resolvePromise, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolvePromise) })
  const port = server.address().port
  await new Promise((resolvePromise) => server.close(resolvePromise))
  return port
}

async function runCommand(commandName, commandArgs, cwd) {
  await new Promise((resolvePromise, reject) => {
    const child = spawn(commandName, commandArgs, { cwd, stdio: 'inherit', windowsHide: true, shell: process.platform === 'win32' })
    child.once('error', reject)
    child.once('close', (code) => code === 0 ? resolvePromise() : reject(new Error(`${commandName} ${commandArgs.join(' ')} failed with ${code}`)))
  })
}

function runCommandSync(commandName, commandArgs, cwd) {
  return execFileSync(commandName, commandArgs, { cwd, encoding: 'utf8', windowsHide: true, shell: process.platform === 'win32' }).trim()
}

function parseOptions(values) {
  const parsed = {}
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index]
    if (value === '--headed') parsed.headed = true
    else if (value.startsWith('--')) parsed[value.slice(2)] = values[++index]
  }
  return parsed
}

function sleep(ms) { return new Promise((resolvePromise) => setTimeout(resolvePromise, ms)) }

main().catch((error) => {
  console.error(error.stack ?? error)
  process.exitCode = 1
})
