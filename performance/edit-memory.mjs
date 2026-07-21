import { spawn } from 'node:child_process'
import { randomBytes } from 'node:crypto'
import { createWriteStream } from 'node:fs'
import { mkdir, mkdtemp, rm, writeFile } from 'node:fs/promises'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'

const options = parseOptions(process.argv.slice(2))
const format = String(options.format ?? '')
if (!['v5', 'v6'].includes(format)) throw new Error('--format must be v5 or v6')

const backend = resolve(String(options.backend ?? ''))
const backendDir = resolve(String(options.cwd ?? resolve(dirname(backend), '..', '..')))
const count = Number(options.count ?? 100_000)
const seed = Number(options.seed ?? 20260718)
const output = resolve(String(options.output ?? join(process.cwd(), `edit-memory-${format}.json`)))
const logPath = resolve(String(options.log ?? join(dirname(output), `${format}.log`)))
const eventsPath = resolve(String(options.events ?? join(dirname(output), `${format}.jsonl`)))
if (!Number.isSafeInteger(count) || count < 1 || count > 1_250_000) {
  throw new Error('--count must be an integer between 1 and 1,250,000')
}
if (!Number.isSafeInteger(seed) || seed < 0) throw new Error('--seed must be a non-negative integer')

await mkdir(dirname(output), { recursive: true })
const root = await mkdtemp(join(tmpdir(), `urocissa-${format}-edit-memory-`))
const token = randomBytes(24).toString('hex')
const password = 'urocissa-edit-memory-benchmark'
const port = await freePort()
let server = null

try {
  await prepareRoot(root, port, password)

  server = await startServer({ backend, backendDir, root, token, eventsPath, logPath, port })
  await setPhase(port, token, 'fixture.insert')
  const fixtureStarted = Date.now()
  const fixture = await perfFetch(port, token, '/__perf/fixture', {
    method: 'POST',
    body: JSON.stringify({ count, seed }),
    headers: { 'content-type': 'application/json' }
  })
  const fixtureWallMs = Date.now() - fixtureStarted
  await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
  await stopServer(server)
  server = null

  const startupStarted = Date.now()
  server = await startServer({ backend, backendDir, root, token, eventsPath, logPath, port })
  const startup = await waitForStatus(
    port,
    token,
    (status) => status.disk_count === count && status.memory_count === count,
    180_000
  )
  const startupWallMs = Date.now() - startupStarted

  const authToken = await requestJson(port, '/post/authenticate', {
    method: 'POST',
    body: JSON.stringify(password),
    headers: { 'content-type': 'application/json' }
  })
  const cookie = `jwt=${authToken}`
  const prefetch = await requestJson(port, '/get/prefetch?', {
    method: 'POST',
    body: 'null',
    headers: { 'content-type': 'application/json', cookie }
  })
  const selectedRecords = Number(prefetch?.prefetch?.dataLength)
  const timestamp = Number(prefetch?.prefetch?.timestamp)
  if (!Number.isSafeInteger(selectedRecords) || selectedRecords < 1 || selectedRecords > count) {
    throw new Error(`invalid prefetch dataLength: ${prefetch?.prefetch?.dataLength}`)
  }
  if (!Number.isSafeInteger(timestamp)) throw new Error(`invalid prefetch timestamp: ${prefetch?.prefetch?.timestamp}`)

  const indexes = Array.from({ length: selectedRecords }, (_, index) => index)
  const editBody = JSON.stringify({ indexArray: indexes, timestamp, isFavorite: true })
  const editBodyBytes = Buffer.byteLength(editBody)
  if (editBodyBytes > 10 * 1024 ** 2) {
    throw new Error(`edit payload is ${editBodyBytes} bytes and exceeds the configured 10 MiB JSON limit`)
  }

  const preEdit = await perfFetch(port, token, '/__perf/status')
  await setPhase(port, token, 'edit-memory.batch-favorite')
  const editStarted = Date.now()
  await requestJson(port, '/put/edit_flags', {
    method: 'PUT',
    body: editBody,
    headers: { 'content-type': 'application/json', cookie }
  })
  let completed = await perfFetch(port, token, '/__perf/barrier', { method: 'POST' })
  if (format === 'v6') {
    completed = await perfFetch(port, token, '/__perf/drain', { method: 'POST' })
  }
  const editWallMs = Date.now() - editStarted

  const result = {
    schemaVersion: 1,
    generatedAt: new Date().toISOString(),
    format,
    backend,
    fixture: {
      requestedRecords: count,
      selectedRecords,
      seed,
      fixtureWallMs,
      startupWallMs,
      databaseBytes: completed.database_bytes,
      editPayloadBytes: editBodyBytes
    },
    workload: {
      operation: 'batch-favorite-true',
      selection: 'explicit-all-prefetch-indexes',
      v5Durability: format === 'v5' ? 'synchronous-redb-write-plus-full-tree-rebuild' : null,
      v6Durability: format === 'v6' ? 'ram-publish-plus-drained-chunked-write-behind' : null,
      editWallMs
    },
    memory: {
      preEditRssBytes: preEdit.backend_rss_bytes,
      editAverageRssBytes: completed.backend_phase_average_rss_bytes,
      editPeakRssBytes: completed.backend_phase_peak_rss_bytes,
      editEndRssBytes: completed.backend_rss_bytes,
      averageIncreaseBytes: completed.backend_phase_average_rss_bytes - preEdit.backend_rss_bytes,
      peakIncreaseBytes: completed.backend_phase_peak_rss_bytes - preEdit.backend_rss_bytes,
      phaseSampleCount: completed.backend_phase_rss_sample_count,
      globalPeakRssBytes: completed.backend_global_peak_rss_bytes
    },
    startup,
    preEdit,
    completed,
    fixtureResponse: fixture
  }
  await writeFile(output, JSON.stringify(result, null, 2))
  process.stdout.write(`${JSON.stringify(result, null, 2)}\n`)
} finally {
  await stopServer(server)
  await rm(root, { recursive: true, force: true })
}

async function prepareRoot(rootPath, serverPort, configPassword) {
  await writeFile(join(rootPath, '.urocissa-performance-root'), 'Disposable edit-memory benchmark root.\n')
  await mkdir(join(rootPath, 'db'), { recursive: true })
  await mkdir(join(rootPath, 'object', 'imported'), { recursive: true })
  await mkdir(join(rootPath, 'object', 'compressed'), { recursive: true })
  await mkdir(join(rootPath, 'upload'), { recursive: true })
  const config = {
    public: {
      address: '127.0.0.1',
      port: serverPort,
      limits: { json: '10MiB', file: '10GiB', 'data-form': '10GiB' },
      syncPaths: [],
      readOnlyMode: false,
      disableImg: true,
      writeBehind: { flushIntervalMs: 1000, softLimitMiB: 16, hardLimitMiB: 32 }
    },
    private: {
      password: configPassword,
      authKey: randomBytes(32).toString('hex'),
      discordHookUrl: null
    }
  }
  await writeFile(join(rootPath, 'config.json'), JSON.stringify(config, null, 2))
}

async function startServer({ backend: binary, backendDir: cwd, root: rootPath, token: perfToken, eventsPath: events, logPath: log, port: serverPort }) {
  const logStream = createWriteStream(log, { flags: 'a' })
  const child = spawn(binary, [], {
    cwd,
    env: {
      ...process.env,
      UROCISSA_PERF_ROOT: rootPath,
      UROCISSA_PERF_TOKEN: perfToken,
      UROCISSA_PERF_EVENTS: events
    },
    stdio: ['ignore', 'pipe', 'pipe'],
    windowsHide: true
  })
  child.stdout.pipe(logStream, { end: false })
  child.stderr.pipe(logStream, { end: false })
  child.on('error', (error) => logStream.write(`\nprocess error: ${error.stack ?? error}\n`))
  const handle = { child, logStream }
  try {
    await waitForStatus(
      serverPort,
      perfToken,
      (status) => status.disk_count >= 0 && status.memory_count >= 0,
      120_000,
      child
    )
    return handle
  } catch (error) {
    await stopServer(handle)
    throw error
  }
}

async function stopServer(handle) {
  if (!handle) return
  if (handle.child.exitCode === null) {
    const closed = new Promise((resolveClose) => handle.child.once('close', resolveClose))
    handle.child.kill()
    const exited = await Promise.race([closed.then(() => true), sleep(5_000).then(() => false)])
    if (!exited) {
      try { handle.child.kill('SIGKILL') } catch {}
    }
  }
  handle.logStream.end()
}

async function waitForStatus(serverPort, perfToken, predicate, timeoutMs, child = null) {
  const deadline = Date.now() + timeoutMs
  let lastError = null
  while (Date.now() < deadline) {
    if (child?.exitCode != null) throw new Error(`backend exited during readiness with code ${child.exitCode}`)
    try {
      const status = await perfFetch(serverPort, perfToken, '/__perf/status')
      if (predicate(status)) return status
    } catch (error) {
      lastError = error
    }
    await sleep(100)
  }
  throw new Error(`server readiness timed out: ${lastError?.message ?? 'unknown error'}`)
}

async function perfFetch(serverPort, perfToken, path, init = {}) {
  return requestJson(serverPort, path, {
    ...init,
    headers: { 'x-urocissa-perf-token': perfToken, ...(init.headers ?? {}) }
  })
}

async function requestJson(serverPort, path, init = {}) {
  const response = await fetch(`http://127.0.0.1:${serverPort}${path}`, init)
  const text = await response.text()
  if (!response.ok) throw new Error(`${init.method ?? 'GET'} ${path} returned ${response.status}: ${text}`)
  return text ? JSON.parse(text) : null
}

async function setPhase(serverPort, perfToken, name) {
  await perfFetch(serverPort, perfToken, '/__perf/phase', {
    method: 'POST',
    body: JSON.stringify({ name }),
    headers: { 'content-type': 'application/json' }
  })
}

async function freePort() {
  const { createServer } = await import('node:net')
  const server = createServer()
  await new Promise((resolvePromise, reject) => {
    server.once('error', reject)
    server.listen(0, '127.0.0.1', resolvePromise)
  })
  const address = server.address()
  const serverPort = typeof address === 'object' && address ? address.port : 0
  await new Promise((resolvePromise) => server.close(resolvePromise))
  return serverPort
}

function parseOptions(values) {
  const parsed = {}
  for (let index = 0; index < values.length; index += 1) {
    const value = values[index]
    if (!value.startsWith('--')) throw new Error(`unexpected argument: ${value}`)
    parsed[value.slice(2)] = values[++index]
  }
  return parsed
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms))
}
