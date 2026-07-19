import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import process from 'node:process'
import { chromium } from 'playwright'

const args = process.argv.slice(2)

function option(name, fallback) {
  const index = args.indexOf(`--${name}`)
  return index === -1 ? fallback : args[index + 1]
}

function numberOption(name, fallback) {
  const value = Number(option(name, fallback))
  if (!Number.isFinite(value) || value <= 0) throw new Error(`--${name} must be positive`)
  return value
}

const config = {
  url: option('url', 'http://127.0.0.1:5173').replace(/\/$/, ''),
  password: option('password', process.env.UROCISSA_PASSWORD ?? 'password'),
  samples: Math.floor(numberOption('samples', 3)),
  pulses: Math.floor(numberOption('pulses', 36)),
  deltaY: numberOption('delta', 12),
  intervalMs: numberOption('interval', 8),
  cpuRate: numberOption('cpu-rate', 1),
  viewportWidth: Math.floor(numberOption('viewport-width', 1920)),
  viewportHeight: Math.floor(numberOption('viewport-height', 1000)),
  scrollWorkPerPulseBudgetMs: numberOption('scroll-work-per-pulse-budget', 0.7),
  behaviorTolerancePx: numberOption('behavior-tolerance', 1),
  output: option('output', null),
  expect: option('expect', 'none'),
  headed: args.includes('--headed')
}

if (!['none', 'janky', 'smooth'].includes(config.expect)) {
  throw new Error('--expect must be one of: none, janky, smooth')
}

const transparentPng = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
  'base64'
)

const traceCategories = [
  'benchmark',
  'blink',
  'cc',
  'devtools.timeline',
  'disabled-by-default-devtools.timeline',
  'disabled-by-default-devtools.timeline.frame',
  'input'
].join(',')

function percentile(values, ratio) {
  if (values.length === 0) return 0
  const sorted = [...values].sort((a, b) => a - b)
  return sorted[Math.min(sorted.length - 1, Math.floor(sorted.length * ratio))]
}

function round(value, digits = 3) {
  const factor = 10 ** digits
  return Math.round(value * factor) / factor
}

function summarizeTrace(events) {
  const rendererMainThreads = new Set(
    events
      .filter(
        (event) =>
          event.ph === 'M' &&
          event.name === 'thread_name' &&
          event.args?.name === 'CrRendererMain'
      )
      .map((event) => `${event.pid}:${event.tid}`)
  )
  const onRendererMain = (event) => rendererMainThreads.has(`${event.pid}:${event.tid}`)
  const durationMs = (event) => (event.dur ?? 0) / 1000
  const scrollEvents = events.filter(
    (event) =>
      event.name === 'EventDispatch' &&
      event.args?.data?.type === 'scroll' &&
      (rendererMainThreads.size === 0 || onRendererMain(event))
  )
  const layoutEvents = events.filter(
    (event) => event.name === 'UpdateLayoutTree' && onRendererMain(event)
  )
  const tasks = events.filter((event) => event.name === 'RunTask' && onRendererMain(event))
  const droppedFrames = events.filter((event) => event.name === 'DroppedFrame')
  const scrollLayers = events.filter(
    (event) =>
      event.name === 'ScrollLayer' &&
      String(event.args?.data?.nodeName ?? '').includes("id='image-container'")
  )
  const scrollDurations = scrollEvents.map(durationMs)
  const layoutDurations = layoutEvents.map(durationMs)
  const taskDurations = tasks.map(durationMs)
  const droppedFrameTimes = droppedFrames.map((event) => event.ts / 1000).sort((a, b) => a - b)
  const droppedFrameIntervals = droppedFrameTimes.slice(1).map((time, index) => time - droppedFrameTimes[index])
  const slowestScrollEvent = scrollEvents.reduce(
    (slowest, event) => (!slowest || (event.dur ?? 0) > (slowest.dur ?? 0) ? event : slowest),
    null
  )
  const slowestScrollEventBreakdown = []
  if (slowestScrollEvent) {
    const eventEnd = slowestScrollEvent.ts + (slowestScrollEvent.dur ?? 0)
    const totals = new Map()
    for (const event of events) {
      if (
        event !== slowestScrollEvent &&
        event.ph === 'X' &&
        event.pid === slowestScrollEvent.pid &&
        event.tid === slowestScrollEvent.tid &&
        event.ts >= slowestScrollEvent.ts &&
        event.ts + (event.dur ?? 0) <= eventEnd
      ) {
        totals.set(event.name, (totals.get(event.name) ?? 0) + durationMs(event))
      }
    }
    slowestScrollEventBreakdown.push(
      ...[...totals.entries()]
        .sort((left, right) => right[1] - left[1])
        .slice(0, 8)
        .map(([name, totalMs]) => ({ name, totalMs: round(totalMs) }))
    )
  }

  return {
    droppedFrameCount: droppedFrames.length,
    droppedFrameIntervalMedianMs: round(percentile(droppedFrameIntervals, 0.5)),
    scrollEventCount: scrollEvents.length,
    scrollEventMaxMs: round(Math.max(0, ...scrollDurations)),
    scrollEventTotalMs: round(scrollDurations.reduce((sum, value) => sum + value, 0)),
    updateLayoutTreeCount: layoutEvents.length,
    updateLayoutTreeMaxMs: round(Math.max(0, ...layoutDurations)),
    updateLayoutTreeTotalMs: round(layoutDurations.reduce((sum, value) => sum + value, 0)),
    runTaskMaxMs: round(Math.max(0, ...taskDurations)),
    longTaskCount: taskDurations.filter((duration) => duration >= 50).length,
    imageContainerScrollLayerCount: scrollLayers.length,
    slowestScrollEventBreakdown
  }
}

function summarizeFrames(state) {
  const frameGaps = state.frames.map((frame) => frame.gap).filter((gap) => gap !== null)
  const trackedFrames = state.frames.filter((frame) => frame.anchorTop !== null)
  const anchorFrames = trackedFrames.filter(
    (frame) => String(frame.anchorStart) === String(state.anchorStart)
  )
  const contentMoves = []
  let stalledInputFrames = 0
  let scrollResetCount = 0

  for (let index = 1; index < trackedFrames.length; index += 1) {
    const previous = trackedFrames[index - 1]
    const current = trackedFrames[index]
    if (previous.anchorStart !== current.anchorStart) continue
    const movement = Math.abs(current.anchorTop - previous.anchorTop)
    contentMoves.push(movement)
    const wheelArrived = state.wheelEvents.some(
      (wheel) => wheel.time > previous.time && wheel.time <= current.time
    )
    if (wheelArrived && movement < 0.5) stalledInputFrames += 1
  }

  for (let index = 1; index < state.scrollEvents.length; index += 1) {
    if (state.scrollEvents[index].scrollTop < state.scrollEvents[index - 1].scrollTop - 0.5) {
      scrollResetCount += 1
    }
  }

  const firstAnchorFrame = anchorFrames[0]
  const lastAnchorFrame = anchorFrames[anchorFrames.length - 1]
  const expectedVisualDisplacementPx = state.wheelEvents.reduce(
    (sum, wheel) => sum + wheel.deltaY,
    0
  )
  const actualVisualDisplacementPx =
    firstAnchorFrame && lastAnchorFrame
      ? firstAnchorFrame.anchorTop - lastAnchorFrame.anchorTop
      : Number.NaN
  const visualDisplacementErrorPx = Number.isFinite(actualVisualDisplacementPx)
    ? Math.abs(actualVisualDisplacementPx - expectedVisualDisplacementPx)
    : Number.POSITIVE_INFINITY

  return {
    frameCount: frameGaps.length,
    frameGapP95Ms: round(percentile(frameGaps, 0.95)),
    frameGapMaxMs: round(Math.max(0, ...frameGaps)),
    frameGapOver25MsCount: frameGaps.filter((gap) => gap >= 25).length,
    visualFrameCount: trackedFrames.length,
    visualMoveP95Px: round(percentile(contentMoves, 0.95)),
    visualMoveMaxPx: round(Math.max(0, ...contentMoves)),
    stalledInputFrames,
    wheelEventCount: state.wheelEvents.length,
    scrollEventCount: state.scrollEvents.length,
    scrollResetCount,
    expectedVisualDisplacementPx: round(expectedVisualDisplacementPx),
    actualVisualDisplacementPx: round(actualVisualDisplacementPx),
    visualDisplacementErrorPx: round(visualDisplacementErrorPx)
  }
}

async function startTrace(page) {
  const session = await page.context().newCDPSession(page)
  const events = []
  session.on('Tracing.dataCollected', ({ value }) => events.push(...value))
  await session.send('Emulation.setCPUThrottlingRate', { rate: config.cpuRate })
  await session.send('Tracing.start', {
    categories: traceCategories,
    options: 'sampling-frequency=10000',
    transferMode: 'ReportEvents'
  })

  return async () => {
    const complete = new Promise((resolveComplete) => {
      session.once('Tracing.tracingComplete', resolveComplete)
    })
    await session.send('Tracing.end')
    await complete
    await session.detach()
    return events
  }
}

async function installInstrumentation(context) {
  await context.addInitScript(() => {
    const state = {
      running: false,
      anchorStart: null,
      frames: [],
      wheelEvents: [],
      scrollEvents: [],
      lastFrame: null
    }
    window.__scrollLag = state

    window.addEventListener(
      'wheel',
      (event) => {
        if (state.running) {
          state.wheelEvents.push({ time: performance.now(), deltaY: event.deltaY })
        }
      },
      { capture: true, passive: true }
    )
    window.addEventListener(
      'scroll',
      (event) => {
        if (state.running && event.target?.id === 'image-container') {
          state.scrollEvents.push({
            time: performance.now(),
            scrollTop: event.target.scrollTop
          })
        }
      },
      true
    )

    const sampleFrame = (time) => {
      if (state.running) {
        const container = document.querySelector('#image-container')
        const anchor =
          state.anchorStart === null
            ? null
            : document.querySelector(`#buffer [start="${state.anchorStart}"]`)
        state.frames.push({
          time,
          gap: state.lastFrame === null ? null : time - state.lastFrame,
          scrollTop: container?.scrollTop ?? null,
          anchorStart: anchor?.getAttribute('start') ?? null,
          anchorTop: anchor?.getBoundingClientRect().top ?? null
        })
        state.lastFrame = time
      }
      requestAnimationFrame(sampleFrame)
    }
    requestAnimationFrame(sampleFrame)
  })
}

async function loginAndWait(page) {
  await page.goto(`${config.url}/login`, { waitUntil: 'domcontentloaded' })
  const password = page.getByRole('textbox', { name: 'Password' })
  if (await password.isVisible().catch(() => false)) {
    await password.fill(config.password)
    await page.getByRole('button', { name: 'Login' }).click()
    await page.waitForURL((url) => url.pathname !== '/login', { timeout: 30_000 })
  }

  const imageContainer = page.locator('#image-container')
  await imageContainer.waitFor({ state: 'visible', timeout: 30_000 })
  try {
    await page.waitForFunction(() => document.querySelectorAll('#buffer [start]').length > 0, null, {
      timeout: 30_000
    })
  } catch (error) {
    const detail = await page.evaluate(() => ({
      url: location.href,
      title: document.title,
      body: document.body?.innerText?.slice(0, 1200) ?? '',
      bufferExists: document.querySelector('#buffer') !== null,
      rowCount: document.querySelectorAll('#buffer [start]').length,
      galleryItemCount: document.querySelectorAll('[data-testid="gallery-item"]').length
    }))
    throw new Error(`gallery rows did not appear: ${JSON.stringify(detail)}`, { cause: error })
  }
  await page.waitForTimeout(1500)
  return imageContainer
}

async function beginFrameSampling(page) {
  return page.evaluate(() => {
    const state = window.__scrollLag
    const viewportMiddle = window.innerHeight / 2
    const rows = [...document.querySelectorAll('#buffer [start]')]
    const anchor = rows.sort(
      (left, right) =>
        Math.abs(left.getBoundingClientRect().top - viewportMiddle) -
        Math.abs(right.getBoundingClientRect().top - viewportMiddle)
    )[0]
    if (!state || !anchor) throw new Error('scroll instrumentation could not find an anchor row')
    state.anchorStart = anchor.getAttribute('start')
    state.frames.length = 0
    state.wheelEvents.length = 0
    state.scrollEvents.length = 0
    state.lastFrame = null
    state.running = true
    return state.anchorStart
  })
}

async function finishFrameSampling(page) {
  return page.evaluate(() => {
    const state = window.__scrollLag
    state.running = false
    return {
      anchorStart: state.anchorStart,
      frames: state.frames,
      wheelEvents: state.wheelEvents,
      scrollEvents: state.scrollEvents
    }
  })
}

async function runSample(browser, sampleIndex) {
  const context = await browser.newContext({
    viewport: { width: config.viewportWidth, height: config.viewportHeight },
    deviceScaleFactor: 1,
    serviceWorkers: 'block'
  })
  // Keep font resolution identical when the baseline frontend is served from a
  // temporary Git-HEAD tree whose node_modules is linked into the workspace.
  await context.route('**/materialdesignicons-webfont.*', async (route) => {
    await route.fulfill({ status: 200, contentType: 'font/woff2', body: Buffer.alloc(0) })
  })
  await context.route('**/object/**', async (route) => {
    await route.fulfill({ status: 200, contentType: 'image/png', body: transparentPng })
  })
  await installInstrumentation(context)
  const page = await context.newPage()
  const errors = []
  page.on('pageerror', (error) => errors.push(`PAGE ${error.message}`))
  page.on('requestfailed', (request) => {
    if (request.failure()?.errorText !== 'net::ERR_ABORTED') {
      errors.push(`REQUEST ${request.url()} ${request.failure()?.errorText ?? 'failed'}`)
    }
  })
  page.on('response', (response) => {
    if (response.status() >= 400 && response.status() !== 401) {
      errors.push(`HTTP ${response.status()} ${response.url()}`)
    }
  })

  try {
    const imageContainer = await loginAndWait(page)
    await imageContainer.hover()
    const stopTrace = await startTrace(page)
    const anchorStart = await beginFrameSampling(page)
    await page.waitForTimeout(100)

    for (let pulse = 0; pulse < config.pulses; pulse += 1) {
      await page.mouse.wheel(0, config.deltaY)
      await page.waitForTimeout(config.intervalMs)
    }
    await page.waitForTimeout(500)

    const frameState = await finishFrameSampling(page)
    const traceEvents = await stopTrace()
    const frameMetrics = summarizeFrames(frameState)
    const traceMetrics = summarizeTrace(traceEvents)
    const scrollEventWorkPerPulseMs = round(
      traceMetrics.scrollEventTotalMs / config.pulses
    )
    const behaviorEquivalent =
      frameMetrics.wheelEventCount === config.pulses &&
      frameMetrics.visualDisplacementErrorPx <= config.behaviorTolerancePx
    const jankSignature =
      scrollEventWorkPerPulseMs >= config.scrollWorkPerPulseBudgetMs ||
      frameMetrics.frameGapOver25MsCount > 0 ||
      traceMetrics.updateLayoutTreeMaxMs >= 8 ||
      traceMetrics.longTaskCount > 0

    return {
      sample: sampleIndex + 1,
      anchorStart,
      behaviorEquivalent,
      jankSignature,
      scrollEventWorkPerPulseMs,
      ...frameMetrics,
      ...traceMetrics,
      errors
    }
  } finally {
    await context.close()
  }
}

function aggregate(samples) {
  const numericKeys = Object.keys(samples[0]).filter((key) =>
    samples.every((sample) => typeof sample[key] === 'number')
  )
  const metrics = {}
  for (const key of numericKeys) {
    const values = samples.map((sample) => sample[key])
    metrics[key] = {
      median: round(percentile(values, 0.5)),
      max: round(Math.max(...values))
    }
  }
  return {
    jankySamples: samples.filter((sample) => sample.jankSignature).length,
    behaviorEquivalentSamples: samples.filter((sample) => sample.behaviorEquivalent).length,
    metrics
  }
}

const browser = await chromium.launch({ headless: !config.headed })
let report
try {
  const samples = []
  for (let sampleIndex = 0; sampleIndex < config.samples; sampleIndex += 1) {
    const sample = await runSample(browser, sampleIndex)
    samples.push(sample)
    console.error(
      `sample ${sample.sample}: dropped=${sample.droppedFrameCount} ` +
        `scrollWorkPerPulse=${sample.scrollEventWorkPerPulseMs}ms ` +
        `behaviorError=${sample.visualDisplacementErrorPx}px`
    )
  }
  report = {
    generatedAt: new Date().toISOString(),
    config: { ...config, password: '<redacted>' },
    aggregate: aggregate(samples),
    samples
  }
} finally {
  await browser.close()
}

if (config.output) {
  const outputPath = resolve(config.output)
  await mkdir(dirname(outputPath), { recursive: true })
  await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8')
}

console.log(JSON.stringify(report, null, 2))

const requiredJankySamples = Math.ceil(config.samples * 0.6)
if (report.aggregate.behaviorEquivalentSamples !== config.samples) {
  throw new Error(
    `scroll behavior changed in ${config.samples - report.aggregate.behaviorEquivalentSamples}/` +
      `${config.samples} samples (tolerance ${config.behaviorTolerancePx}px)`
  )
}
const runtimeErrors = report.samples.flatMap((sample) => sample.errors)
if (runtimeErrors.length > 0) {
  throw new Error(`browser errors were observed:\n${runtimeErrors.join('\n')}`)
}
if (config.expect === 'janky' && report.aggregate.jankySamples < requiredJankySamples) {
  throw new Error(
    `expected reproducible jank in at least ${requiredJankySamples}/${config.samples} samples, ` +
      `observed ${report.aggregate.jankySamples}`
  )
}
if (config.expect === 'smooth' && report.aggregate.jankySamples >= requiredJankySamples) {
  throw new Error(
    `expected smooth scrolling in a majority of samples, observed ` +
      `${report.aggregate.jankySamples}/${config.samples} jank signatures`
  )
}
