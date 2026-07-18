import { chromium } from 'playwright'
import { spawn } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { existsSync, createWriteStream } from 'node:fs'
import { execFileSync } from 'node:child_process'
import { mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..')
const backendDir = join(repoRoot, 'gallery-backend')
const frontendDir = join(repoRoot, 'gallery-frontend')
const backendBinary = join(
  backendDir,
  'target',
  'release',
  process.platform === 'win32' ? 'urocissa.exe' : 'urocissa'
)
const artifactRoot = join(repoRoot, '.performance')
const defaultSeed = 20260718n
const defaultCount = 100_000
const defaultSamples = 3
const viewport = { width: 1440, height: 900 }

const args = process.argv.slice(2)
const command = args.shift() ?? 'smoke'
const options = parseOptions(args)

if (!['baseline', 'compare', 'smoke'].includes(command)) {
  console.error(`Unknown command: ${command}`)
  process.exit(2)
}

const count = Number(options.count ?? (command === 'smoke' ? 1_000 : defaultCount))
const samples = Number(options.samples ?? (command === 'smoke' ? 1 : defaultSamples))
const seed = BigInt(options.seed ?? defaultSeed)
if (!Number.isSafeInteger(count) || count < 1 || count > 2_000_000) throw new Error('count must be an integer between 1 and 2,000,000')
if (!Number.isSafeInteger(samples) || samples < 1) throw new Error('samples must be a positive integer')
if (seed < 0n || seed > BigInt(Number.MAX_SAFE_INTEGER)) throw new Error('seed must be between 0 and Number.MAX_SAFE_INTEGER for JSON/u64 interoperability')
const headed = options.headed === true

async function main() {
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
    current.comparison = comparison
    await writeFile(join(resultDir, 'summary.json'), JSON.stringify(current, null, 2))
    await writeFile(join(resultDir, 'report.md'), renderReport(current, comparison))
    console.log(renderConsoleSummary(current, comparison))
    if (!current.correctness.ok) process.exitCode = 1
    return
  }

  const resultDir = command === 'baseline'
    ? join(artifactRoot, 'baseline', 'latest')
    : join(artifactRoot, 'smoke', timestamp())
  const summary = await runSuite({ resultDir, count, samples, seed, headed })
  await writeFile(join(resultDir, 'summary.json'), JSON.stringify(summary, null, 2))
  await writeFile(join(resultDir, 'report.md'), renderReport(summary))
  console.log(renderConsoleSummary(summary))
  if (!summary.correctness.ok) process.exitCode = 1
}

async function ensureBuilds() {
  await runCommand('cargo', ['build', '--release', '--features', 'performance-test'], backendDir)
  if (!existsSync(join(frontendDir, 'node_modules'))) {
    await runCommand('npm', ['ci'], frontendDir)
  }
  await runCommand('npm', ['run', 'build:only'], frontendDir)
}

async function runSuite({ resultDir, count, samples, seed, headed }) {
  await mkdir(join(resultDir, 'samples'), { recursive: true })
  const summary = {
    schemaVersion: 2,
    generatedAt: new Date().toISOString(),
    source: sourceIdentity(),
    environment: {
      platform: process.platform,
      arch: process.arch,
      node: process.version,
      viewport,
      browser: 'chromium',
      buildProfile: 'release',
      fixture: { count, samples, seed: seed.toString() }
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
      disableImg: true
    },
    private: { password: 'urocissa-performance-password', authKey: randomBytes(32).toString('hex'), discordHookUrl: null }
  }
  await writeFile(join(root, 'config.json'), JSON.stringify(config, null, 2))

  const firstEvents = join(sampleDir, 'backend-seed.jsonl')
  const firstLog = join(sampleDir, 'backend-seed.log')
  let first = null
  let second = null
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
    await stopServer(first)
    first = null

    const secondEvents = join(sampleDir, 'backend-runtime.jsonl')
    const secondLog = join(sampleDir, 'backend-runtime.log')
    const startupStart = Date.now()
    second = await startServer({ port, token, root, events: secondEvents, logPath: secondLog })
    const startup = await waitForStatus(port, token, (value) => value.disk_count === count && value.memory_count === count)
    const startupWallMs = Date.now() - startupStart

    const browser = await runBrowserJourney({ port, token, sampleDir, sampleIndex, headed })

    await setPhase(port, token, 'delete')
    const deleteStart = Date.now()
    const deleteSummary = await perfFetch(port, token, '/__perf/fixture', { method: 'DELETE' })
    const deleteWallMs = Date.now() - deleteStart
    const finalStatus = await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
    await stopServer(second)
    second = null
    await rm(root, { recursive: true, force: true })

    const correctness = checkCorrectness({
      count,
      seedResponse,
      startup,
      browser,
      deleteSummary,
      finalStatus,
      rootExists: existsSync(root)
    })

    const events = [...await readEvents(firstEvents), ...await readEvents(secondEvents)]
    return {
      sampleIndex,
      seedWallMs,
      startupWallMs,
      fixture: seedResponse,
      startup,
      browser,
      delete: { ...deleteSummary, wallMs: deleteWallMs },
      finalStatus,
      backendEvents: events,
      correctness
    }
  } finally {
    await stopServer(second)
    await stopServer(first)
    await rm(root, { recursive: true, force: true })
  }
}

async function runBrowserJourney({ port, token, sampleDir, sampleIndex, headed }) {
  const baseUrl = `http://localhost:${port}`
  const browser = await chromium.launch({ headless: !headed })
  const context = await browser.newContext({
    viewport
  })
  const page = await context.newPage()
  const responses = []
  const errors = []
  let authComplete = false
  let currentPhase = 'startup'
  const requestStarts = new WeakMap()
  page.on('request', (request) => {
    if (request.url().includes('/get/') || request.url().includes('/post/')) requestStarts.set(request, Date.now())
  })
  page.on('response', (response) => {
    if (response.url().includes('/get/') || response.url().includes('/post/')) {
      const request = response.request()
      const started = requestStarts.get(request)
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
  page.on('requestfailed', (request) => errors.push(`REQUEST ${request.url()} ${request.failure()?.errorText ?? 'failed'}`))
  page.on('pageerror', (error) => { if (!error.message.includes('status code 401')) errors.push(`PAGE ${error.message}`) })
  page.on('console', (message) => {
    const text = message.text()
    if (message.type() === 'error' && !text.includes('No Worker is Working') && (authComplete || !text.includes('401'))) errors.push(`CONSOLE ${text}`)
  })

  await context.addInitScript(() => {
    const state = { longTasks: [], paints: [], frameGaps: [], lastFrame: null }
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
    await waitForRequestQuiet(page, 350)
    await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
    const browserMetrics = await readBrowserMetrics(page)
    phases.push({ name, wallMs: Date.now() - start, responseCount: responses.length - beforeResponses, browserMetrics, payload })
    return payload
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

  await phase('warm-reload', async () => {
    const prefetch = page.waitForResponse((response) => response.url().includes('/get/prefetch') && response.status() === 200)
    await page.reload({ waitUntil: 'domcontentloaded' })
    return await (await prefetch).json()
  })

  const navigation = await page.evaluate(() => {
    const navigationEntry = performance.getEntriesByType('navigation')[0]
    return navigationEntry ? {
      domContentLoaded: navigationEntry.domContentLoadedEventEnd,
      load: navigationEntry.loadEventEnd,
      response: navigationEntry.responseEnd
    } : null
  })
  await page.screenshot({ path: join(sampleDir, `browser-${sampleIndex}.png`), fullPage: false })
  return { phases, navigation, responses, errors }
  } finally {
    await context.close().catch(() => {})
    await browser.close().catch(() => {})
  }
}

async function readBrowserMetrics(page) {
  return page.evaluate(() => {
    const state = window.__urocissaPerf ?? { longTasks: [], paints: [], frameGaps: [] }
    const sortedGaps = [...state.frameGaps].sort((a, b) => a - b)
    const percentile = (values, p) => values.length ? values[Math.min(values.length - 1, Math.floor(values.length * p))] : 0
    return {
      longTaskCount: state.longTasks.length,
      longTaskTotalMs: state.longTasks.reduce((sum, value) => sum + value, 0),
      longTaskMaxMs: Math.max(0, ...state.longTasks),
      paintEntries: state.paints,
      frameCount: state.frameGaps.length,
      frameGapP95Ms: percentile(sortedGaps, 0.95),
      frameGapMaxMs: Math.max(0, ...state.frameGaps),
      heapUsedBytes: performance.memory?.usedJSHeapSize ?? null
    }
  })
}

async function readEvents(path) {
  if (!existsSync(path)) return []
  const text = await readFile(path, 'utf8')
  return text.split(/\r?\n/).filter(Boolean).map((line) => JSON.parse(line))
}

function aggregateSamples(samples) {
  const values = new Map()
  for (const sample of samples) {
    for (const phase of sample.browser?.phases ?? []) addValue(values, `browser.${phase.name}.wallMs`, phase.wallMs)
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
    addValue(values, 'server.startup.wallMs', sample.startupWallMs)
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

function compareSummaries(baseline, current) {
  if (baseline.schemaVersion !== current.schemaVersion) {
    throw new Error(`incompatible benchmark schemas: baseline=${baseline.schemaVersion}, current=${current.schemaVersion}`)
  }
  const keys = new Set([...Object.keys(baseline.aggregates ?? {}), ...Object.keys(current.aggregates ?? {})])
  const metrics = {}
  for (const key of keys) {
    const before = baseline.aggregates?.[key]?.median ?? null
    const after = current.aggregates?.[key]?.median ?? null
    const delta = before && after ? after - before : null
    const relative = before && after ? delta / before : null
    const threshold = key.startsWith('browser.') ? 10 : 1
    metrics[key] = { baseline: before, current: after, delta, relative, notable: relative != null && delta >= threshold && relative >= 0.1 }
  }
  return { metrics, notableRegressions: Object.entries(metrics).filter(([, value]) => value.notable).map(([key]) => key) }
}

function renderReport(summary, comparison = null) {
  const lines = [`# Urocissa performance report`, '', `- Generated: ${summary.generatedAt}`, `- Samples: ${summary.samples.length}`, `- Fixture: ${summary.environment.fixture.count} records`, `- Correctness: ${summary.correctness.ok ? 'PASS' : 'FAIL'}`, '']
  if (summary.correctness.errors.length) lines.push('## Correctness errors', '', ...summary.correctness.errors.map((error) => `- ${error}`), '')
  lines.push('## Aggregate timings', '', '| Metric | Median | P95 | Max |', '|---|---:|---:|---:|')
  for (const [key, value] of Object.entries(summary.aggregates ?? {})) lines.push(`| ${key} | ${formatMs(value.median)} | ${formatMs(value.p95)} | ${formatMs(value.max)} |`)
  if (comparison) {
    lines.push('', '## Baseline comparison', '', '| Metric | Baseline | Current | Delta |', '|---|---:|---:|---:|')
    for (const [key, value] of Object.entries(comparison.metrics)) lines.push(`| ${key} | ${formatMs(value.baseline)} | ${formatMs(value.current)} | ${formatDelta(value.delta, value.relative)} |`)
    lines.push('', `Notable regressions: ${comparison.notableRegressions.length || 'none'}`)
  }
  return `${lines.join('\n')}\n`
}

function renderConsoleSummary(summary, comparison = null) {
  const lines = [`correctness: ${summary.correctness.ok ? 'PASS' : 'FAIL'}`, `samples: ${summary.samples.length}`, `fixture: ${summary.environment.fixture.count}`]
  if (comparison) lines.push(`notable timing regressions: ${comparison.notableRegressions.length || 'none'}`)
  return lines.join('\n')
}

function checkCorrectness({ count, seedResponse, startup, browser, deleteSummary, finalStatus, rootExists }) {
  const errors = []
  if (seedResponse?.inserted !== count) errors.push(`inserted ${seedResponse?.inserted ?? 'unknown'} of ${count}`)
  if (startup?.disk_count !== count || startup?.memory_count !== count) errors.push('restart readiness count mismatch')
  if (!browser || browser.errors?.length) errors.push(...(browser?.errors ?? ['browser journey did not complete']))
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

function formatMs(value) { return value == null ? '-' : `${value.toFixed(3)} ms` }
function formatDelta(delta, relative) { return delta == null ? '-' : `${delta >= 0 ? '+' : ''}${delta.toFixed(3)} ms (${(relative * 100).toFixed(1)}%)` }
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

async function waitForRequestQuiet(page, quietMs) {
  let last = Date.now()
  const listener = (response) => { if (response.url().includes('/get/')) last = Date.now() }
  page.on('response', listener)
  while (Date.now() - last < quietMs) await sleep(50)
  page.off('response', listener)
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
