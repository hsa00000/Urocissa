import { spawn } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { mkdir, writeFile } from 'node:fs/promises'
import { dirname, resolve } from 'node:path'
import process from 'node:process'
import { createInterface } from 'node:readline'
import { fileURLToPath } from 'node:url'
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

function nonNegativeNumberOption(name, fallback) {
  const value = Number(option(name, fallback))
  if (!Number.isFinite(value) || value < 0) throw new Error(`--${name} must be non-negative`)
  return value
}

function signedNumberOption(name, fallback) {
  const value = Number(option(name, fallback))
  if (!Number.isFinite(value) || value === 0) throw new Error(`--${name} must be non-zero`)
  return value
}

const supportedScenarios = new Set([
  'continuous-down',
  'discrete-wheel',
  'discrete-wheel-delay',
  'cdp-gesture',
  'cdp-gesture-delay',
  'cdp-discrete-gesture',
  'thumbnail-throughput',
  'thumbnail-burst-during-scroll',
  'desktop-interaction',
  'mobile-interaction',
  'native-wheel',
  'native-wheel-delay',
  'continuous-up',
  'worker-delay',
  'bounds',
  'scrollbar',
  'locate',
  'resize',
  'mobile'
])

const config = {
  url: option('url', 'http://127.0.0.1:5173').replace(/\/$/, ''),
  password: option('password', process.env.UROCISSA_PASSWORD ?? 'password'),
  browser: option('browser', 'chromium'),
  samples: Math.floor(numberOption('samples', 3)),
  pulses: Math.floor(numberOption('pulses', 36)),
  deltaY: numberOption('delta', 12),
  intervalMs: numberOption('interval', 8),
  pulseSettleMs: numberOption('pulse-settle', 150),
  nativeWheelDelta: signedNumberOption('os-wheel-delta', -120),
  cpuRate: numberOption('cpu-rate', 1),
  viewportWidth: Math.floor(numberOption('viewport-width', 1920)),
  viewportHeight: Math.floor(numberOption('viewport-height', 1000)),
  scrollWorkPerPulseBudgetMs: numberOption('scroll-work-per-pulse-budget', 0.7),
  behaviorTolerancePx: numberOption('behavior-tolerance', 1),
  timerZeroBudget: Math.floor(nonNegativeNumberOption('timer-zero-budget', 10)),
  workerDelayMs: Math.floor(numberOption('worker-delay', 300)),
  thumbnailDelayMs: Math.floor(numberOption('thumbnail-delay', 40)),
  scenario: option('scenario', 'continuous-down'),
  locate: option('locate', null),
  output: option('output', null),
  expect: option('expect', 'none'),
  headed: args.includes('--headed')
}

if (!supportedScenarios.has(config.scenario)) {
  throw new Error(`--scenario must be one of: ${[...supportedScenarios].join(', ')}`)
}

if (!['none', 'janky', 'smooth', 'strict-smooth'].includes(config.expect)) {
  throw new Error('--expect must be one of: none, janky, smooth, strict-smooth')
}

const transparentPng = Buffer.from(
  'iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII=',
  'base64'
)

const nativeWheelScript = resolve(
  dirname(fileURLToPath(import.meta.url)),
  'native-wheel-input.ps1'
)
const thumbnailBurstSize = 20

function withTimeout(promise, timeoutMs, message) {
  let timer
  const timeout = new Promise((_, reject) => {
    timer = setTimeout(() => reject(new Error(message)), timeoutMs)
  })
  return Promise.race([promise, timeout]).finally(() => clearTimeout(timer))
}

function createObjectRouteState(delayMs) {
  let mode = 'immediate'
  let holdLimit = Number.POSITIVE_INFINITY
  let releaseHeldRequests = null
  let heldRequestGate = Promise.resolve()
  const state = {
    requestCount: 0,
    completedRequestCount: 0,
    activeRequestCount: 0,
    peakConcurrentRequestCount: 0,
    heldRequestCount: 0,
    lastActivityAt: Date.now()
  }

  return {
    async handle(route) {
      state.requestCount += 1
      state.activeRequestCount += 1
      state.peakConcurrentRequestCount = Math.max(
        state.peakConcurrentRequestCount,
        state.activeRequestCount
      )
      state.lastActivityAt = Date.now()

      if (mode === 'hold' && state.heldRequestCount < holdLimit) {
        state.heldRequestCount += 1
        await heldRequestGate
      }
      if (delayMs > 0) {
        await new Promise((resolveDelay) => setTimeout(resolveDelay, delayMs))
      }

      try {
        await route.fulfill({ status: 200, contentType: 'image/png', body: transparentPng })
      } finally {
        state.activeRequestCount -= 1
        state.completedRequestCount += 1
        state.lastActivityAt = Date.now()
      }
    },
    armHold(limit = Number.POSITIVE_INFINITY) {
      mode = 'hold'
      holdLimit = limit
      state.heldRequestCount = 0
      heldRequestGate = new Promise((resolveHeld) => {
        releaseHeldRequests = resolveHeld
      })
    },
    releaseHeld() {
      mode = 'immediate'
      holdLimit = Number.POSITIVE_INFINITY
      releaseHeldRequests?.()
      releaseHeldRequests = null
    },
    async waitForHeld(minimum, timeoutMs = 3000) {
      const deadline = Date.now() + timeoutMs
      while (Date.now() < deadline && state.heldRequestCount < minimum) {
        await new Promise((resolvePoll) => setTimeout(resolvePoll, 25))
      }
      return state.heldRequestCount
    },
    async waitForQuiet({ minimumRequests = 1, quietMs = 350, timeoutMs = 15_000 } = {}) {
      const deadline = Date.now() + timeoutMs
      while (Date.now() < deadline) {
        if (
          state.requestCount >= minimumRequests &&
          state.activeRequestCount === 0 &&
          Date.now() - state.lastActivityAt >= quietMs
        ) {
          return
        }
        await new Promise((resolvePoll) => setTimeout(resolvePoll, 25))
      }
      throw new Error(
        `thumbnail requests did not become quiet: ${JSON.stringify(this.snapshot())}`
      )
    },
    snapshot() {
      return { ...state }
    }
  }
}

async function launchNativeWheelController(page) {
  if (process.platform !== 'win32') {
    throw new Error('native-wheel scenarios require Windows')
  }

  const originalTitle = await page.title()
  const titleToken = `urocissa-native-wheel-${randomUUID()}`
  await page.evaluate((title) => {
    document.title = title
  }, titleToken)

  const helper = spawn(
    'powershell.exe',
    [
      '-NoLogo',
      '-NoProfile',
      '-NonInteractive',
      '-ExecutionPolicy',
      'Bypass',
      '-File',
      nativeWheelScript,
      '-WindowTitleToken',
      titleToken
    ],
    { stdio: ['pipe', 'pipe', 'pipe'], windowsHide: true }
  )
  const lines = createInterface({ input: helper.stdout })
  const iterator = lines[Symbol.asyncIterator]()
  let helperError = null
  let stderr = ''
  helper.on('error', (error) => {
    helperError = error
  })
  helper.stderr.on('data', (chunk) => {
    stderr += chunk.toString()
  })

  const readMessage = async () => {
    const result = await withTimeout(
      iterator.next(),
      15_000,
      `native wheel helper timed out${stderr ? `: ${stderr.trim()}` : ''}`
    )
    if (result.done) {
      throw new Error(
        `native wheel helper exited${helperError ? `: ${helperError.message}` : ''}` +
          `${stderr ? `: ${stderr.trim()}` : ''}`
      )
    }
    try {
      return JSON.parse(result.value)
    } catch (error) {
      throw new Error(`native wheel helper returned invalid JSON: ${result.value}`, {
        cause: error
      })
    }
  }

  const ready = await readMessage()
  if (ready.type !== 'ready') {
    throw new Error(`native wheel helper did not become ready: ${JSON.stringify(ready)}`)
  }

  return {
    ready,
    async wheel(delta) {
      helper.stdin.write(`wheel ${delta}\n`)
      const result = await readMessage()
      if (result.type !== 'wheel' || result.sent !== 1 || !result.foreground) {
        throw new Error(`native wheel input was refused: ${JSON.stringify(result)}`)
      }
      return result
    },
    async close() {
      if (helper.exitCode === null) {
        helper.stdin.write('quit\n')
        helper.stdin.end()
        await withTimeout(
          new Promise((resolveExit) => helper.once('exit', resolveExit)),
          5_000,
          'native wheel helper did not exit'
        ).catch(() => helper.kill())
      }
      lines.close()
      await page
        .evaluate((title) => {
          document.title = title
        }, originalTitle)
        .catch(() => {})
    }
  }
}

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
  const scrollEndEvents = events.filter(
    (event) =>
      event.name === 'EventDispatch' &&
      event.args?.data?.type === 'scrollend' &&
      (rendererMainThreads.size === 0 || onRendererMain(event))
  )
  const layoutEvents = events.filter(
    (event) => event.name === 'UpdateLayoutTree' && onRendererMain(event)
  )
  const tasks = events.filter((event) => event.name === 'RunTask' && onRendererMain(event))
  const handlePostMessages = events.filter(
    (event) => event.name === 'HandlePostMessage' && onRendererMain(event)
  )
  const timerInstalls = events.filter((event) => {
    if (event.name !== 'TimerInstall' || !onRendererMain(event)) return false
    const stack = event.args?.data?.stackTrace ?? []
    return !stack.some((frame) => String(frame.url ?? '').startsWith('chrome-extension://'))
  })
  const timerDelayById = new Map(
    timerInstalls.map((event) => [
      `${event.pid}:${event.tid}:${event.args?.data?.timerId}`,
      Number(event.args?.data?.timeout ?? -1)
    ])
  )
  const timerFires = events.filter(
    (event) => event.name === 'TimerFire' && onRendererMain(event)
  )
  const timerInstallCount = (delay) =>
    timerInstalls.filter((event) => Number(event.args?.data?.timeout) === delay).length
  const timerFireCount = (delay) =>
    timerFires.filter(
      (event) =>
        timerDelayById.get(`${event.pid}:${event.tid}:${event.args?.data?.timerId}`) === delay
    ).length
  const droppedFrames = events.filter((event) => event.name === 'DroppedFrame')
  const scrollLayers = events.filter(
    (event) =>
      event.name === 'ScrollLayer' &&
      String(event.args?.data?.nodeName ?? '').includes("id='image-container'")
  )
  const scrollDurations = scrollEvents.map(durationMs)
  const scrollEndDurations = scrollEndEvents.map(durationMs)
  const layoutDurations = layoutEvents.map(durationMs)
  const taskDurations = tasks.map(durationMs)
  const handlePostMessageDurations = handlePostMessages.map(durationMs)
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
  const slowestRunTask = tasks.reduce(
    (slowest, event) =>
      !slowest || (event.dur ?? 0) > (slowest.dur ?? 0) ? event : slowest,
    null
  )
  const slowestRunTaskBreakdown =
    slowestRunTask === null
      ? []
      : events
          .filter(
            (event) =>
              event !== slowestRunTask &&
              event.ph === 'X' &&
              event.pid === slowestRunTask.pid &&
              event.tid === slowestRunTask.tid &&
              event.ts >= slowestRunTask.ts &&
              event.ts + (event.dur ?? 0) <=
                slowestRunTask.ts + (slowestRunTask.dur ?? 0)
          )
          .sort((left, right) => (right.dur ?? 0) - (left.dur ?? 0))
          .slice(0, 12)
          .map((event) => ({
            name: event.name,
            durationMs: round(durationMs(event)),
            eventType: event.args?.data?.type ?? null,
            functionName: event.args?.data?.functionName ?? null,
            url: event.args?.data?.url ?? null
          }))

  const animationFrames = events
    .filter((event) => event.name === 'FireAnimationFrame' && onRendererMain(event))
    .sort((left, right) => left.ts - right.ts)
  const traceFrameGapAnalysis = []
  for (let index = 1; index < animationFrames.length; index += 1) {
    const previousFrame = animationFrames[index - 1]
    const currentFrame = animationFrames[index]
    const gapMs = (currentFrame.ts - previousFrame.ts) / 1000
    if (gapMs < 25) continue

    const overlappingEvents = events.filter(
      (event) =>
        event.ph === 'X' &&
        onRendererMain(event) &&
        event.ts < currentFrame.ts &&
        event.ts + (event.dur ?? 0) > previousFrame.ts
    )
    const eventsByName = new Map()
    for (const event of overlappingEvents) {
      if (event.name === 'FireAnimationFrame') continue
      const summary = eventsByName.get(event.name) ?? {
        name: event.name,
        count: 0,
        totalMs: 0,
        maxMs: 0
      }
      const eventDurationMs = durationMs(event)
      summary.count += 1
      summary.totalMs += eventDurationMs
      summary.maxMs = Math.max(summary.maxMs, eventDurationMs)
      eventsByName.set(event.name, summary)
    }

    traceFrameGapAnalysis.push({
      gapMs: round(gapMs),
      runTaskMaxMs: round(
        Math.max(
          0,
          ...overlappingEvents
            .filter((event) => event.name === 'RunTask')
            .map(durationMs)
        )
      ),
      topEvents: [...eventsByName.values()]
        .sort((left, right) => right.maxMs - left.maxMs)
        .slice(0, 12)
        .map((summary) => ({
          ...summary,
          totalMs: round(summary.totalMs),
          maxMs: round(summary.maxMs)
        }))
    })
  }

  return {
    droppedFrameCount: droppedFrames.length,
    droppedFrameIntervalMedianMs: round(percentile(droppedFrameIntervals, 0.5)),
    scrollEventCount: scrollEvents.length,
    scrollEventMaxMs: round(Math.max(0, ...scrollDurations)),
    scrollEventTotalMs: round(scrollDurations.reduce((sum, value) => sum + value, 0)),
    scrollEndEventCount: scrollEndEvents.length,
    scrollEndEventMaxMs: round(Math.max(0, ...scrollEndDurations)),
    scrollEndEventTotalMs: round(scrollEndDurations.reduce((sum, value) => sum + value, 0)),
    updateLayoutTreeCount: layoutEvents.length,
    updateLayoutTreeMaxMs: round(Math.max(0, ...layoutDurations)),
    updateLayoutTreeTotalMs: round(layoutDurations.reduce((sum, value) => sum + value, 0)),
    runTaskMaxMs: round(Math.max(0, ...taskDurations)),
    handlePostMessageCount: handlePostMessages.length,
    handlePostMessageTotalMs: round(
      handlePostMessageDurations.reduce((sum, value) => sum + value, 0)
    ),
    handlePostMessageMaxMs: round(Math.max(0, ...handlePostMessageDurations)),
    longTaskCount: taskDurations.filter((duration) => duration >= 50).length,
    timerInstallCount: timerInstalls.length,
    timerFireCount: timerFires.length,
    timerInstall0Count: timerInstallCount(0),
    timerInstall50Count: timerInstallCount(50),
    timerInstall75Count: timerInstallCount(75),
    timerInstall100Count: timerInstallCount(100),
    timerFire0Count: timerFireCount(0),
    timerFire50Count: timerFireCount(50),
    timerFire75Count: timerFireCount(75),
    timerFire100Count: timerFireCount(100),
    imageContainerScrollLayerCount: scrollLayers.length,
    slowestScrollEventBreakdown,
    slowestRunTaskBreakdown,
    traceAnimationFrameCount: animationFrames.length,
    traceFrameGapAnalysis
  }
}

function summarizeFrames(state, expectedDisplacementOverride = null) {
  const frameGaps = state.frames.map((frame) => frame.gap).filter((gap) => gap !== null)
  const firstScrollTime = state.scrollEvents[0]?.time ?? null
  const lastScrollEndTime = state.scrollEndEvents.at(-1)?.time ?? null
  const frameGapDetails = []
  for (let index = 1; index < state.frames.length; index += 1) {
    const previous = state.frames[index - 1]
    const current = state.frames[index]
    const gap = current.time - previous.time
    if (gap < 25) continue

    const scrollEventCount = state.scrollEvents.filter(
      (event) => event.time > previous.time && event.time <= current.time
    ).length
    const scrollEndEventCount = state.scrollEndEvents.filter(
      (event) => event.time > previous.time && event.time <= current.time
    ).length
    const thumbnailMutationCount = (state.thumbnailMutations ?? []).filter(
      (mutation) => mutation.time > previous.time && mutation.time <= current.time
    ).length

    frameGapDetails.push({
      gapMs: round(gap),
      startTime: round(previous.time),
      endTime: round(current.time),
      duringScrollTransaction:
        firstScrollTime !== null &&
        lastScrollEndTime !== null &&
        current.time >= firstScrollTime &&
        previous.time <= lastScrollEndTime,
      scrollEventCount,
      scrollEndEventCount,
      thumbnailMutationCount
    })
  }
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
  const expectedVisualDisplacementPx =
    expectedDisplacementOverride ??
    state.wheelEvents.reduce((sum, wheel) => sum + wheel.deltaY, 0)
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
    frameGapDetails,
    visualFrameCount: trackedFrames.length,
    visualMoveP95Px: round(percentile(contentMoves, 0.95)),
    visualMoveMaxPx: round(Math.max(0, ...contentMoves)),
    stalledInputFrames,
    wheelEventCount: state.wheelEvents.length,
    scrollEventCount: state.scrollEvents.length,
    domScrollEndEventCount: state.scrollEndEvents.length,
    scrollResetCount,
    expectedVisualDisplacementPx: round(expectedVisualDisplacementPx),
    actualVisualDisplacementPx: round(actualVisualDisplacementPx),
    visualDisplacementErrorPx: round(visualDisplacementErrorPx)
  }
}

function summarizePassiveFrames(state) {
  return {
    ...summarizeFrames(state, 0),
    visualFrameCount: 0,
    visualMoveP95Px: 0,
    visualMoveMaxPx: 0,
    stalledInputFrames: 0,
    expectedVisualDisplacementPx: 0,
    actualVisualDisplacementPx: 0,
    visualDisplacementErrorPx: 0
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
  await context.addInitScript(
    `globalThis.__trackThumbnailMutations = ${config.scenario === 'thumbnail-throughput'}`
  )
  await context.addInitScript(() => {
    const trackThumbnailMutations = globalThis.__trackThumbnailMutations === true
    const state = {
      running: false,
      anchorStart: null,
      frames: [],
      wheelEvents: [],
      scrollEvents: [],
      scrollEndEvents: [],
      thumbnailMutations: [],
      thumbnailFrame: 0,
      samplingStartedAt: 0,
      lastFrame: null,
      trackVisual: true
    }
    window.__scrollLag = state

    window.addEventListener(
      'wheel',
      (event) => {
        if (state.running) {
          const wheelRecord = {
            time: performance.now(),
            deltaX: event.deltaX,
            deltaY: event.deltaY,
            deltaMode: event.deltaMode,
            wheelDelta: event.wheelDelta,
            wheelDeltaY: event.wheelDeltaY,
            cancelable: event.cancelable,
            defaultPrevented: event.defaultPrevented,
            isTrusted: event.isTrusted,
            targetId: event.target instanceof Element ? event.target.id || null : null,
            insideImageContainer:
              event.target instanceof Element && Boolean(event.target.closest('#image-container'))
          }
          state.wheelEvents.push(wheelRecord)
          queueMicrotask(() => {
            wheelRecord.defaultPrevented = event.defaultPrevented
          })
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
    window.addEventListener(
      'scrollend',
      (event) => {
        if (state.running && event.target?.id === 'image-container') {
          state.scrollEndEvents.push({
            time: performance.now(),
            scrollTop: event.target.scrollTop
          })
        }
      },
      true
    )

    if (trackThumbnailMutations) {
      const thumbnailObserver = new MutationObserver((records) => {
        for (const record of records) {
          const target = record.target
          if (
            record.attributeName !== 'src' ||
            !(target instanceof HTMLImageElement) ||
            !target.matches('.desktop-small-image, .mobile-small-image') ||
            !target.getAttribute('src')?.startsWith('blob:')
          ) {
            continue
          }

          const index = Number(target.dataset.itemIndex)
          if (!Number.isInteger(index)) continue
          state.thumbnailMutations.push({
            index,
            time: performance.now(),
            frame: state.thumbnailFrame
          })
        }
      })
      thumbnailObserver.observe(document, {
        attributes: true,
        subtree: true,
        attributeFilter: ['src']
      })
    }

    const sampleFrame = (time) => {
      state.thumbnailFrame += 1
      if (state.running) {
        const container = state.trackVisual
          ? document.querySelector('#image-container')
          : null
        const anchor =
          state.trackVisual && state.anchorStart !== null
            ? document.querySelector(`#buffer [start="${state.anchorStart}"]`)
            : null
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

async function waitForGallery(page) {
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

async function loginAndWait(page) {
  const response = await page.context().request.post(`${config.url}/post/authenticate`, {
    data: JSON.stringify(config.password),
    headers: {
      'Content-Type': 'application/json'
    },
    timeout: 10_000
  })
  if (!response.ok()) {
    throw new Error(`login failed with HTTP ${response.status()}`)
  }

  const token = await response.json()
  if (typeof token !== 'string' || token.length === 0) {
    throw new Error('login response did not contain a JWT string')
  }
  await page.context().addCookies([
    {
      name: 'jwt',
      value: token,
      url: config.url,
      sameSite: 'Strict'
    }
  ])

  await page.goto(`${config.url}/home`, {
    waitUntil: 'domcontentloaded',
    timeout: 15_000
  })
  return waitForGallery(page)
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
    state.scrollEndEvents.length = 0
    state.lastFrame = null
    state.samplingStartedAt = performance.now()
    state.trackVisual = true
    state.running = true
    return state.anchorStart
  })
}

async function beginPassiveSampling(page) {
  return page.evaluate(() => {
    const state = window.__scrollLag
    if (!state) throw new Error('scroll instrumentation was not installed')
    state.anchorStart = null
    state.frames.length = 0
    state.wheelEvents.length = 0
    state.scrollEvents.length = 0
    state.scrollEndEvents.length = 0
    state.lastFrame = null
    state.samplingStartedAt = performance.now()
    state.trackVisual = false
    state.running = true
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
      scrollEvents: state.scrollEvents,
      scrollEndEvents: state.scrollEndEvents,
      thumbnailMutations: state.thumbnailMutations.filter(
        (mutation) => mutation.time >= state.samplingStartedAt
      )
    }
  })
}

async function waitForScrollTransactionCommit(
  page,
  physicalAnchor,
  timeout = Math.max(3000, config.pulseSettleMs * 4)
) {
  await page.waitForFunction(
    ({ anchor, tolerance }) => {
      const state = window.__scrollLag
      const container = document.querySelector('#image-container')
      const lastWheel = state?.wheelEvents.at(-1)
      if (!state || !(container instanceof HTMLElement) || lastWheel === undefined) {
        return false
      }

      const nativeScrollEnded = state.scrollEndEvents.some(
        (scrollEnd) => scrollEnd.time >= lastWheel.time
      )
      return nativeScrollEnded && Math.abs(container.scrollTop - anchor) <= tolerance
    },
    { anchor: physicalAnchor, tolerance: config.behaviorTolerancePx },
    { timeout }
  )
}

async function waitForPostScrollWork(page, delayMs = 0) {
  if (delayMs > 0) {
    await page.waitForTimeout(delayMs)
  }
  await page.evaluate(
    ({ quietMs, deadlineMs }) =>
      new Promise((resolveQuiet) => {
        const root = document.querySelector('#buffer')
        if (root === null) {
          resolveQuiet()
          return
        }

        let quietTimer
        let deadlineTimer
        const observer = new MutationObserver(() => {
          clearTimeout(quietTimer)
          quietTimer = setTimeout(finish, quietMs)
        })
        const finish = () => {
          clearTimeout(quietTimer)
          clearTimeout(deadlineTimer)
          observer.disconnect()
          resolveQuiet()
        }

        observer.observe(root, {
          attributes: true,
          childList: true,
          subtree: true,
          attributeFilter: ['class', 'src', 'style']
        })
        quietTimer = setTimeout(finish, quietMs)
        deadlineTimer = setTimeout(finish, deadlineMs)
      }),
    { quietMs: 300, deadlineMs: 5000 }
  )
  await page.evaluate(
    () =>
      new Promise((resolveFrames) => {
        requestAnimationFrame(() => requestAnimationFrame(resolveFrames))
      })
  )
}

async function readGalleryImageState(page) {
  return page.evaluate(() => {
    const shells = [...document.querySelectorAll('.desktop-small-image, .mobile-small-image')]
    const vueApp = document.querySelector('#app')?.__vue_app__
    const pinia = vueApp?.config?.globalProperties?.$pinia
    const constStore = [...(pinia?._s?.values?.() ?? [])].find(
      (store) => store.$id === 'constStoremainId'
    )
    return {
      hardwareConcurrency: navigator.hardwareConcurrency,
      configuredImageConcurrency: constStore?.concurrencyNumber ?? null,
      galleryItemCount: document.querySelectorAll('[data-testid="gallery-item"]').length,
      imageShellCount: shells.length,
      loadedImageShellCount: shells.filter(
        (image) =>
          image instanceof HTMLImageElement && image.getAttribute('src')?.startsWith('blob:')
      ).length,
      hiddenImageShellCount: shells.filter(
        (image) => image instanceof HTMLElement && getComputedStyle(image).display === 'none'
      ).length
    }
  })
}

async function readThumbnailAppearance(page) {
  return page.evaluate(() => {
    const state = window.__scrollLag
    const firstRow = document.querySelector('#buffer [start]')
    const firstRowIndexes = new Set(
      [...(firstRow?.querySelectorAll('.desktop-small-image, .mobile-small-image') ?? [])]
        .map((element) => Number(element.getAttribute('data-item-index')))
        .filter(Number.isInteger)
    )
    const firstMutationByIndex = new Map()
    for (const mutation of state?.thumbnailMutations ?? []) {
      if (!firstRowIndexes.has(mutation.index) || firstMutationByIndex.has(mutation.index)) {
        continue
      }
      firstMutationByIndex.set(mutation.index, mutation)
    }

    const mutations = [...firstMutationByIndex.values()].sort((left, right) => left.time - right.time)
    const frameCounts = new Map()
    for (const mutation of mutations) {
      frameCounts.set(mutation.frame, (frameCounts.get(mutation.frame) ?? 0) + 1)
    }
    const firstMutation = mutations[0]
    const lastMutation = mutations.at(-1)
    const ninetyPercentMutation =
      mutations[Math.max(0, Math.ceil(mutations.length * 0.9) - 1)]
    const withinThreeFrames =
      firstMutation === undefined
        ? 0
        : mutations.filter((mutation) => mutation.frame - firstMutation.frame <= 2).length

    return {
      firstRowShellCount: firstRowIndexes.size,
      observedThumbnailCount: mutations.length,
      appearanceFrameCount: frameCounts.size,
      singleThumbnailFrameCount: [...frameCounts.values()].filter((count) => count === 1).length,
      withinThreeFrameRatio:
        mutations.length === 0 ? 0 : withinThreeFrames / mutations.length,
      firstToNinetyPercentMs:
        firstMutation === undefined || ninetyPercentMutation === undefined
          ? Number.POSITIVE_INFINITY
          : ninetyPercentMutation.time - firstMutation.time,
      firstToLastMs:
        firstMutation === undefined || lastMutation === undefined
          ? Number.POSITIVE_INFINITY
          : lastMutation.time - firstMutation.time,
      frameCounts: [...frameCounts.entries()].map(([frame, count]) => ({ frame, count }))
    }
  })
}

async function firstRenderedRowStart(page) {
  return page.evaluate(
    () => document.querySelector('#buffer [start]')?.getAttribute('start') ?? null
  )
}

async function renderedRowCount(page) {
  return page.evaluate(() => document.querySelectorAll('#buffer [start]').length)
}

async function waitForDifferentRow(page, previousStart, timeout = 5000) {
  await page.waitForFunction(
    (start) => {
      const rows = [...document.querySelectorAll('#buffer [start]')]
      return rows.length > 0 && rows[0]?.getAttribute('start') !== start
    },
    previousStart,
    { timeout }
  )
  return firstRenderedRowStart(page)
}

async function clickScrollbarAt(page, percentage) {
  const scrollbar = page.locator('#scroll-bar')
  const box = await scrollbar.boundingBox()
  if (!box) throw new Error('scrollbar does not have a bounding box')
  await page.mouse.click(
    box.x + box.width / 2,
    box.y + Math.min(Math.max(box.height * percentage, 1), box.height - 1)
  )
}

async function dragScrollbar(page, fromPercentage, toPercentage) {
  const scrollbar = page.locator('#scroll-bar')
  const box = await scrollbar.boundingBox()
  if (!box) throw new Error('scrollbar does not have a bounding box')
  const x = box.x + box.width / 2
  await page.mouse.move(x, box.y + box.height * fromPercentage)
  await page.mouse.down()
  await page.mouse.move(x, box.y + box.height * toPercentage, { steps: 8 })
  await page.mouse.up()
}

async function runWheelPulses(page, deltaY) {
  for (let pulse = 0; pulse < config.pulses; pulse += 1) {
    await page.mouse.wheel(0, deltaY)
    await page.waitForTimeout(config.intervalMs)
  }
}

if (!['chromium', 'chrome'].includes(config.browser)) {
  throw new Error('--browser must be one of: chromium, chrome')
}

async function captureWheelAnchor(page, anchorStart = null) {
  return page.evaluate((requestedStart) => {
    const rows = [...document.querySelectorAll('#buffer [start]')]
    const viewportMiddle = window.innerHeight / 2
    const anchor =
      (requestedStart === null
        ? null
        : document.querySelector(`#buffer [start="${requestedStart}"]`)) ??
      rows.sort(
        (left, right) =>
          Math.abs(left.getBoundingClientRect().top - viewportMiddle) -
          Math.abs(right.getBoundingClientRect().top - viewportMiddle)
      )[0]
    const container = document.querySelector('#image-container')
    const visibleRows = document.querySelector('#buffer .buffer-visible-rows')
    const vueApp = document.querySelector('#app')?.__vue_app__
    const pinia = vueApp?.config?.globalProperties?.$pinia
    const scrollTopStore = [...(pinia?._s?.values?.() ?? [])].find((store) =>
      String(store.$id ?? '').startsWith('scrollTopStore')
    )

    if (!(anchor instanceof HTMLElement) || !(container instanceof HTMLElement)) {
      return null
    }

    return {
      anchorStart: anchor.getAttribute('start'),
      anchorTop: anchor.getBoundingClientRect().top,
      physicalScrollTop: container.scrollTop,
      virtualScrollTop: scrollTopStore?.scrollTop ?? null,
      visibleRowsTop:
        visibleRows instanceof HTMLElement ? visibleRows.getBoundingClientRect().top : null,
      visibleRowsTransform:
        visibleRows instanceof HTMLElement ? visibleRows.style.transform : null,
      rowTransform: anchor.style.transform
    }
  }, anchorStart)
}

async function runDiscreteWheelScenario(page, imageContainer, delayRows = false) {
  if (delayRows) {
    await page.route('**/get/get-rows**', async (route) => {
      await new Promise((resolveDelay) => setTimeout(resolveDelay, config.workerDelayMs))
      await route.continue()
    })
  }

  await imageContainer.hover()
  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  await beginPassiveSampling(page)
  const pulses = []

  for (let pulse = 0; pulse < config.pulses; pulse += 1) {
    const before = await captureWheelAnchor(page)
    if (before === null) throw new Error('discrete wheel could not find an anchor row')
    const wheelEventStartIndex = await page.evaluate(
      () => window.__scrollLag?.wheelEvents.length ?? 0
    )

    await page.mouse.wheel(0, config.deltaY)
    const afterDispatch = await captureWheelAnchor(page, before.anchorStart)
    await page.evaluate(() => new Promise(requestAnimationFrame))
    const afterFrame = await captureWheelAnchor(page, before.anchorStart)
    await page.waitForTimeout(config.pulseSettleMs)
    const afterSettle = await captureWheelAnchor(page, before.anchorStart)
    const inputEvents = await page.evaluate(
      (startIndex) => window.__scrollLag?.wheelEvents.slice(startIndex) ?? [],
      wheelEventStartIndex
    )

    pulses.push({
      pulse: pulse + 1,
      expectedDeltaY: config.deltaY,
      inputEvents,
      wheelPhaseDisplacementPx:
        afterFrame === null ? null : round(before.anchorTop - afterFrame.anchorTop),
      noInputDriftPx:
        afterFrame === null || afterSettle === null
          ? null
          : round(afterFrame.anchorTop - afterSettle.anchorTop),
      actualDisplacementPx:
        afterSettle === null ? null : round(before.anchorTop - afterSettle.anchorTop),
      before,
      afterDispatch,
      afterFrame,
      afterSettle
    })
  }

  await waitForScrollTransactionCommit(page, physicalAnchorStart)
  await page.waitForTimeout(150)
  const frameState = await finishFrameSampling(page)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)
  const invalidPulses = pulses.filter(
    (pulse) =>
      pulse.inputEvents.some((inputEvent) => inputEvent.defaultPrevented) ||
      pulse.actualDisplacementPx === null ||
      Math.abs(pulse.actualDisplacementPx - config.deltaY) > config.behaviorTolerancePx
  )

  return {
    anchorStart: null,
    behaviorEquivalent:
      frameState.wheelEvents.length === config.pulses &&
      invalidPulses.length === 0 &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: true,
    interactionCount: config.pulses,
    details: {
      workerDelayMs: delayRows ? config.workerDelayMs : 0,
      pulseSettleMs: config.pulseSettleMs,
      invalidPulseCount: invalidPulses.length,
      pulses
    }
  }
}

async function runNativeWheelScenario(page, imageContainer, delayRows = false) {
  if (delayRows) {
    await page.route('**/get/get-rows**', async (route) => {
      await new Promise((resolveDelay) => setTimeout(resolveDelay, config.workerDelayMs))
      await route.continue()
    })
  }

  const centerTargetsGallery = await page.evaluate(() => {
    const target = document.elementFromPoint(window.innerWidth / 2, window.innerHeight / 2)
    return target instanceof Element && Boolean(target.closest('#image-container'))
  })
  if (!centerTargetsGallery) {
    throw new Error('the center of the isolated Chrome window is not inside #image-container')
  }

  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  await beginPassiveSampling(page)
  const controller = await launchNativeWheelController(page)
  const pulses = []

  try {
    for (let pulse = 0; pulse < config.pulses; pulse += 1) {
      const cadenceStartedAt = Date.now()
      const before = await captureWheelAnchor(page)
      if (before === null) throw new Error('native wheel could not find an anchor row')
      const pulseFrameStartIndex = await page.evaluate((anchorStart) => {
        const state = window.__scrollLag
        if (!state) return 0
        state.anchorStart = anchorStart
        return state.frames.length
      }, before.anchorStart)
      const wheelEventStartIndex = await page.evaluate(
        () => window.__scrollLag?.wheelEvents.length ?? 0
      )
      const scrollEventStartIndex = await page.evaluate(
        () => window.__scrollLag?.scrollEvents.length ?? 0
      )
      const scrollEndEventStartIndex = await page.evaluate(
        () => window.__scrollLag?.scrollEndEvents.length ?? 0
      )

      const nativeInput = await controller.wheel(config.nativeWheelDelta)
      await page.waitForFunction(
        (startIndex) => (window.__scrollLag?.wheelEvents.length ?? 0) > startIndex,
        wheelEventStartIndex,
        { timeout: 2000 }
      )
      const afterDispatch = await captureWheelAnchor(page, before.anchorStart)
      await page.evaluate(() => new Promise(requestAnimationFrame))
      const afterFrame = await captureWheelAnchor(page, before.anchorStart)
      const remainingCadenceMs = Math.max(
        0,
        config.pulseSettleMs - (Date.now() - cadenceStartedAt)
      )
      if (remainingCadenceMs > 0) {
        await page.waitForTimeout(remainingCadenceMs)
      }
      const afterSettle = await captureWheelAnchor(page, before.anchorStart)
      const inputEvents = await page.evaluate(
        (startIndex) => window.__scrollLag?.wheelEvents.slice(startIndex) ?? [],
        wheelEventStartIndex
      )
      const scrollEvents = await page.evaluate(
        (startIndex) => window.__scrollLag?.scrollEvents.slice(startIndex) ?? [],
        scrollEventStartIndex
      )
      const scrollEndEvents = await page.evaluate(
        (startIndex) => window.__scrollLag?.scrollEndEvents.slice(startIndex) ?? [],
        scrollEndEventStartIndex
      )
      const pulseFrames = await page.evaluate(
        (startIndex) => window.__scrollLag?.frames.slice(startIndex) ?? [],
        pulseFrameStartIndex
      )
      const pulseFrameDisplacements = pulseFrames
        .filter(
          (frame) =>
            frame.anchorTop !== null && String(frame.anchorStart) === String(before.anchorStart)
        )
        .map((frame) => before.anchorTop - frame.anchorTop)
      const nativeMovementSteps = []
      let previousAnchorTop = before.anchorTop
      for (const frame of pulseFrames) {
        if (
          frame.anchorTop === null ||
          String(frame.anchorStart) !== String(before.anchorStart)
        ) {
          continue
        }
        const movement = previousAnchorTop - frame.anchorTop
        if (Math.abs(movement) >= 0.5) nativeMovementSteps.push(movement)
        previousAnchorTop = frame.anchorTop
      }
      const expectedDomDisplacementPx = inputEvents.reduce(
        (sum, inputEvent) => sum + inputEvent.deltaY,
        0
      )
      const expectedDirection = Math.sign(expectedDomDisplacementPx)
      let prematurePhysicalResetCount = 0
      for (let index = 1; index < scrollEvents.length; index += 1) {
        const physicalMovement = scrollEvents[index].scrollTop - scrollEvents[index - 1].scrollTop
        if (physicalMovement * expectedDirection >= -config.behaviorTolerancePx) continue

        const scrollEndedBeforeReset = scrollEndEvents.some(
          (scrollEnd) => scrollEnd.time <= scrollEvents[index].time
        )
        if (!scrollEndedBeforeReset) prematurePhysicalResetCount += 1
      }

      pulses.push({
        pulse: pulse + 1,
        osWheelDelta: config.nativeWheelDelta,
        nativeInput,
        cadenceElapsedMs: Date.now() - cadenceStartedAt,
        expectedDomDisplacementPx: round(expectedDomDisplacementPx),
        inputEvents,
        scrollEvents,
        scrollEndEvents,
        nativeMovementFrameCount: nativeMovementSteps.length,
        nativeMovementStepMaxPx: round(
          Math.max(0, ...nativeMovementSteps.map((movement) => Math.abs(movement)))
        ),
        reverseMovementFrameCount: nativeMovementSteps.filter(
          (movement) => movement * expectedDirection < -config.behaviorTolerancePx
        ).length,
        prematurePhysicalResetCount,
        transientDisplacementMinPx: round(Math.min(0, ...pulseFrameDisplacements)),
        transientDisplacementMaxPx: round(Math.max(0, ...pulseFrameDisplacements)),
        pulseFrames,
        wheelPhaseDisplacementPx:
          afterFrame === null ? null : round(before.anchorTop - afterFrame.anchorTop),
        noInputDriftPx:
          afterFrame === null || afterSettle === null
            ? null
            : round(afterFrame.anchorTop - afterSettle.anchorTop),
        actualDisplacementPx:
          afterSettle === null ? null : round(before.anchorTop - afterSettle.anchorTop),
        before,
        afterDispatch,
        afterFrame,
        afterSettle
      })
    }
    await waitForScrollTransactionCommit(page, physicalAnchorStart)
  } finally {
    await controller.close()
  }

  const finalDriftBefore = await captureWheelAnchor(page)
  await page.waitForTimeout(Math.max(1000, config.pulseSettleMs * 2))
  const finalDriftAfter = await captureWheelAnchor(page, finalDriftBefore?.anchorStart ?? null)
  const finalNoInputDriftPx =
    finalDriftBefore === null || finalDriftAfter === null
      ? null
      : round(finalDriftBefore.anchorTop - finalDriftAfter.anchorTop)
  const frameState = await finishFrameSampling(page)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)
  const invalidPulses = pulses.filter(
    (pulse) =>
      pulse.inputEvents.length !== 1 ||
      pulse.inputEvents.some(
        (inputEvent) =>
          !inputEvent.isTrusted ||
          !inputEvent.insideImageContainer ||
          inputEvent.defaultPrevented
      ) ||
      pulse.nativeMovementFrameCount < 2 ||
      pulse.reverseMovementFrameCount > 0 ||
      pulse.prematurePhysicalResetCount > 0 ||
      pulse.actualDisplacementPx === null ||
      pulse.transientDisplacementMinPx <
        Math.min(0, pulse.expectedDomDisplacementPx) - config.behaviorTolerancePx ||
      pulse.transientDisplacementMaxPx >
        Math.max(0, pulse.expectedDomDisplacementPx) + config.behaviorTolerancePx ||
      Math.abs(pulse.actualDisplacementPx - pulse.expectedDomDisplacementPx) >
        config.behaviorTolerancePx
  )

  return {
    anchorStart: null,
    behaviorEquivalent:
      frameState.wheelEvents.length === config.pulses &&
      invalidPulses.length === 0 &&
      Math.abs(finalNoInputDriftPx ?? Number.POSITIVE_INFINITY) <=
        config.behaviorTolerancePx &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: true,
    scrollBudgetUnit: 'scroll-event',
    interactionCount: config.pulses,
    details: {
      inputSource: 'Windows SendInput MOUSEEVENTF_WHEEL',
      isolatedChromePid: controller.ready.chromePid,
      workerDelayMs: delayRows ? config.workerDelayMs : 0,
      pulseCadenceMs: config.pulseSettleMs,
      finalNoInputDriftPx,
      invalidPulseCount: invalidPulses.length,
      pulses
    }
  }
}

async function runContinuousScenario(page, imageContainer, direction) {
  if (direction < 0) {
    await imageContainer.hover()
    await page.mouse.wheel(0, config.deltaY * config.pulses * 2)
    await page.waitForTimeout(300)
  }

  await imageContainer.hover()
  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  const anchorStart = await beginFrameSampling(page)
  await page.waitForTimeout(100)
  await runWheelPulses(page, config.deltaY * direction)
  await waitForScrollTransactionCommit(page, physicalAnchorStart)
  const frameState = await finishFrameSampling(page)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const frameMetrics = summarizeFrames(frameState)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)

  return {
    anchorStart,
    behaviorEquivalent:
      frameMetrics.wheelEventCount === config.pulses &&
      frameMetrics.visualDisplacementErrorPx <= config.behaviorTolerancePx &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics,
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: true,
    interactionCount: config.pulses,
    details: { direction: direction < 0 ? 'up' : 'down' }
  }
}

function summarizeTimingFrames(frameState, expectedDisplacementPx, before, after) {
  const metrics = summarizePassiveFrames(frameState)
  const actualDisplacementPx =
    before === null || after === null
      ? Number.NaN
      : before.anchorTop - after.anchorTop
  const visualDisplacementErrorPx = Number.isFinite(actualDisplacementPx)
    ? Math.abs(actualDisplacementPx - expectedDisplacementPx)
    : Number.POSITIVE_INFINITY

  return {
    ...metrics,
    visualFrameCount: before === null || after === null ? 0 : 2,
    expectedVisualDisplacementPx: round(expectedDisplacementPx),
    actualVisualDisplacementPx: round(actualDisplacementPx),
    visualDisplacementErrorPx: round(visualDisplacementErrorPx)
  }
}

async function runCdpScrollGestureScenario(page, imageContainer, delayRows = false) {
  if (delayRows) {
    await page.route('**/get/get-rows**', async (route) => {
      await new Promise((resolveDelay) => setTimeout(resolveDelay, config.workerDelayMs))
      await route.continue()
    })
  }

  await imageContainer.hover()
  const box = await imageContainer.boundingBox()
  if (box === null) throw new Error('CDP scroll gesture could not locate #image-container')

  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  const before = await captureWheelAnchor(page)
  if (before === null) throw new Error('CDP scroll gesture could not find its anchor row')
  await beginPassiveSampling(page)
  const distancePx = config.deltaY * config.pulses
  const speedPxPerSecond = (config.deltaY * 1000) / config.intervalMs
  const session = await page.context().newCDPSession(page)

  await page.waitForTimeout(100)
  try {
    await session.send('Input.synthesizeScrollGesture', {
      x: Math.round(box.x + box.width / 2),
      y: Math.round(box.y + box.height / 2),
      yDistance: -distancePx,
      speed: speedPxPerSecond,
      gestureSourceType: 'mouse',
      preventFling: true,
      interactionMarkerName: 'urocissa-frame-gap'
    })
  } finally {
    await session.detach()
  }

  await waitForScrollTransactionCommit(page, physicalAnchorStart)
  await waitForPostScrollWork(
    page,
    delayRows ? config.workerDelayMs + 100 : 0
  )
  const frameState = await finishFrameSampling(page)
  const after = await captureWheelAnchor(page, before.anchorStart)
  const frameMetrics = summarizeTimingFrames(frameState, distancePx, before, after)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)

  return {
    anchorStart: before.anchorStart,
    behaviorEquivalent:
      frameMetrics.visualDisplacementErrorPx <= config.behaviorTolerancePx &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics,
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: true,
    scrollBudgetUnit: 'scroll-event',
    interactionCount: Math.max(frameState.scrollEvents.length, 1),
    details: {
      inputSource: 'CDP Input.synthesizeScrollGesture',
      distancePx,
      speedPxPerSecond,
      workerDelayMs: delayRows ? config.workerDelayMs : 0
    }
  }
}

async function runThumbnailThroughputScenario(page, objectRouteState) {
  await beginPassiveSampling(page)
  const pendingAlbumShellState = await page.evaluate(() => {
    const pendingAlbumShells = [
      ...document.querySelectorAll('img.thumbnail-image--album:not([src])')
    ]
    return {
      pendingCount: pendingAlbumShells.length,
      hiddenCount: pendingAlbumShells.filter(
        (element) => element.hidden || getComputedStyle(element).display === 'none'
      ).length,
      visibleCount: pendingAlbumShells.filter(
        (element) => !element.hidden && getComputedStyle(element).display !== 'none'
      ).length
    }
  })
  await objectRouteState.waitForQuiet({ minimumRequests: 4 })
  await page.evaluate(
    () => new Promise((resolveFrames) => requestAnimationFrame(() => requestAnimationFrame(resolveFrames)))
  )
  const appearance = await readThumbnailAppearance(page)
  const requestState = objectRouteState.snapshot()
  const frameState = await finishFrameSampling(page)
  const observedEnoughThumbnails =
    appearance.observedThumbnailCount >= Math.min(4, appearance.firstRowShellCount) &&
    appearance.observedThumbnailCount >= Math.floor(appearance.firstRowShellCount * 0.8)
  const throughputEquivalent =
    pendingAlbumShellState.visibleCount === 0 &&
    requestState.peakConcurrentRequestCount >= 4 &&
    observedEnoughThumbnails &&
    appearance.withinThreeFrameRatio >= 0.8 &&
    appearance.firstToNinetyPercentMs <= 250 &&
    appearance.singleThumbnailFrameCount <= 2

  return {
    anchorStart: null,
    behaviorEquivalent: throughputEquivalent,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: Math.max(requestState.requestCount, 1),
    details: {
      inputSource: 'controlled thumbnail responses',
      thumbnailDelayMs: config.thumbnailDelayMs,
      throughputEquivalent,
      pendingAlbumShellState,
      requestState,
      appearance
    }
  }
}

async function prepareThumbnailBurstScenario(page, objectRouteState) {
  await objectRouteState.waitForQuiet({ minimumRequests: 4 })
  const initialStart = await firstRenderedRowStart(page)
  objectRouteState.armHold(thumbnailBurstSize)
  await clickScrollbarAt(page, 0.6)
  let finalStart = initialStart
  try {
    finalStart = await waitForDifferentRow(page, initialStart, 5000)
  } catch {
    finalStart = await firstRenderedRowStart(page)
  }
  const heldRequestCount = await objectRouteState.waitForHeld(thumbnailBurstSize, 3000)
  // Requests outside the one-row burst complete before tracing so this test
  // measures a 20-tile RowBlock response burst instead of every overscan row.
  await waitForPostScrollWork(page, 0)
  return { initialStart, finalStart, heldRequestCount, burstSize: thumbnailBurstSize }
}

async function runThumbnailBurstDuringScrollScenario(
  page,
  imageContainer,
  objectRouteState,
  preparation
) {
  if (preparation === null) {
    throw new Error('thumbnail burst scenario was not prepared before tracing')
  }

  const box = await imageContainer.boundingBox()
  if (box === null) throw new Error('thumbnail burst gesture could not locate #image-container')
  // Keep pointer-driven hover transitions out of this thumbnail-specific
  // trace. The CDP gesture still targets the gallery coordinates below.
  await page.mouse.move(0, 0)
  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  const before = await captureWheelAnchor(page)
  if (before === null) throw new Error('thumbnail burst gesture could not find its anchor row')
  await beginPassiveSampling(page)
  const distancePx = config.deltaY * config.pulses
  const speedPxPerSecond = (config.deltaY * 1000) / config.intervalMs
  const session = await page.context().newCDPSession(page)

  try {
    const gesture = session.send('Input.synthesizeScrollGesture', {
      x: Math.round(box.x + box.width / 2),
      y: Math.round(box.y + box.height / 2),
      yDistance: -distancePx,
      speed: speedPxPerSecond,
      gestureSourceType: 'mouse',
      preventFling: true,
      interactionMarkerName: 'urocissa-thumbnail-burst'
    })
    await page.waitForTimeout(50)
    objectRouteState.releaseHeld()
    await gesture
  } finally {
    objectRouteState.releaseHeld()
    await session.detach()
  }

  await waitForScrollTransactionCommit(page, physicalAnchorStart)
  await objectRouteState.waitForQuiet({ minimumRequests: 4 })
  await waitForPostScrollWork(page)
  const frameState = await finishFrameSampling(page)
  const after = await captureWheelAnchor(page, before.anchorStart)
  const frameMetrics = summarizeTimingFrames(frameState, distancePx, before, after)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)
  const requestState = objectRouteState.snapshot()

  return {
    anchorStart: before.anchorStart,
    behaviorEquivalent:
      preparation.heldRequestCount >= 4 &&
      frameMetrics.visualDisplacementErrorPx <= config.behaviorTolerancePx &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics,
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    // This adversarial scenario isolates whether a simultaneous thumbnail
    // completion burst creates a visible frame gap. The main cdp-gesture gate
    // separately enforces the scroll-work and RunTask budgets.
    applyScrollBudget: false,
    interactionCount: Math.max(frameState.scrollEvents.length, 1),
    details: {
      inputSource: 'CDP gesture plus simultaneous thumbnail release',
      distancePx,
      speedPxPerSecond,
      preparation,
      requestState
    }
  }
}

async function runCdpDiscreteGestureScenario(page, imageContainer) {
  await imageContainer.hover()
  const box = await imageContainer.boundingBox()
  if (box === null) throw new Error('CDP discrete gesture could not locate #image-container')

  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  await beginPassiveSampling(page)
  const speedPxPerSecond = (config.deltaY * 1000) / config.intervalMs
  const session = await page.context().newCDPSession(page)
  const pulses = []

  try {
    for (let pulse = 0; pulse < config.pulses; pulse += 1) {
      const cadenceStartedAt = Date.now()
      const before = await captureWheelAnchor(page)
      if (before === null) throw new Error('CDP discrete gesture could not find an anchor row')
      const wheelEventStartIndex = await page.evaluate(
        () => window.__scrollLag?.wheelEvents.length ?? 0
      )

      await session.send('Input.synthesizeScrollGesture', {
        x: Math.round(box.x + box.width / 2),
        y: Math.round(box.y + box.height / 2),
        yDistance: -config.deltaY,
        speed: speedPxPerSecond,
        gestureSourceType: 'mouse',
        preventFling: true,
        interactionMarkerName: `urocissa-short-scroll-${pulse + 1}`
      })
      await waitForScrollTransactionCommit(page, physicalAnchorStart)
      const afterCommit = await captureWheelAnchor(page, before.anchorStart)

      const remainingCadenceMs = Math.max(
        config.pulseSettleMs - (Date.now() - cadenceStartedAt),
        0
      )
      if (remainingCadenceMs > 0) {
        await page.waitForTimeout(remainingCadenceMs)
      }
      const afterSettle = await captureWheelAnchor(page, before.anchorStart)
      const inputEvents = await page.evaluate(
        (startIndex) => window.__scrollLag?.wheelEvents.slice(startIndex) ?? [],
        wheelEventStartIndex
      )

      pulses.push({
        pulse: pulse + 1,
        inputEvents,
        actualDisplacementPx:
          afterSettle === null ? null : round(before.anchorTop - afterSettle.anchorTop),
        noInputDriftPx:
          afterCommit === null || afterSettle === null
            ? null
            : round(afterCommit.anchorTop - afterSettle.anchorTop)
      })
    }
  } finally {
    await session.detach()
  }

  await waitForPostScrollWork(page)
  const frameState = await finishFrameSampling(page)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)
  const invalidPulses = pulses.filter(
    (pulse) =>
      pulse.inputEvents.length === 0 ||
      pulse.inputEvents.some((inputEvent) => inputEvent.defaultPrevented) ||
      pulse.actualDisplacementPx === null ||
      Math.abs(pulse.actualDisplacementPx - config.deltaY) > config.behaviorTolerancePx ||
      Math.abs(pulse.noInputDriftPx ?? Number.POSITIVE_INFINITY) >
        config.behaviorTolerancePx
  )

  return {
    anchorStart: null,
    behaviorEquivalent:
      invalidPulses.length === 0 &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: false,
    interactionCount: config.pulses,
    details: {
      inputSource: 'CDP Input.synthesizeScrollGesture',
      pulseCadenceMs: config.pulseSettleMs,
      invalidPulseCount: invalidPulses.length,
      pulses
    }
  }
}

async function runDesktopInteractionScenario(page) {
  const firstTile = page.locator('[data-testid="gallery-item"]').first()
  await firstTile.waitFor({ state: 'visible', timeout: 10_000 })
  const initialIndex = await firstTile.getAttribute('data-item-index')

  await Promise.all([
    page.waitForURL((url) => url.pathname.includes('/home/view/'), { timeout: 10_000 }),
    firstTile.click()
  ])
  const openedItem = page.url().includes('/home/view/')

  await page.goBack({ waitUntil: 'domcontentloaded', timeout: 10_000 })
  await waitForGallery(page)
  await beginPassiveSampling(page)

  const selectionTile = page.locator('[data-testid="gallery-item"]').first()
  const selectionIndex = await selectionTile.getAttribute('data-item-index')
  if (selectionIndex === null) {
    throw new Error('desktop interaction tile is missing data-item-index')
  }

  await selectionTile.hover()
  const selectButton = page.locator(
    `[data-testid="select-item"][data-item-index="${selectionIndex}"]`
  )
  await selectButton.waitFor({ state: 'attached', timeout: 5_000 })
  const selectIcon = selectButton.locator('.v-icon')
  await selectIcon.waitFor({ state: 'visible', timeout: 5_000 })
  await selectIcon.click()
  await page.waitForFunction(
    (index) =>
      document
        .querySelector(`[data-testid="gallery-item"][data-item-index="${index}"]`)
        ?.classList.contains('gallery-tile--selected') === true,
    selectionIndex,
    { timeout: 5_000 }
  )

  const frameState = await finishFrameSampling(page)
  return {
    anchorStart: null,
    behaviorEquivalent: openedItem && initialIndex !== null,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 2,
    details: {
      inputSource: 'Playwright isolated Chromium pointer interaction',
      openedItem,
      selectedItem: true,
      initialIndex,
      selectionIndex
    }
  }
}

async function runMobileInteractionScenario(page) {
  await beginPassiveSampling(page)
  const firstTile = page.locator('[data-testid="gallery-item"]').first()
  await firstTile.waitFor({ state: 'visible', timeout: 10_000 })
  const index = await firstTile.getAttribute('data-item-index')
  if (index === null) {
    throw new Error('mobile interaction tile is missing data-item-index')
  }

  await firstTile.dispatchEvent('pointerdown', {
    pointerId: 1,
    pointerType: 'touch',
    button: 0,
    buttons: 1,
    isPrimary: true
  })
  await page.waitForTimeout(650)
  await firstTile.dispatchEvent('pointerup', {
    pointerId: 1,
    pointerType: 'touch',
    button: 0,
    buttons: 0,
    isPrimary: true
  })
  await page.waitForFunction(
    (itemIndex) =>
      document
        .querySelector(`[data-testid="gallery-item"][data-item-index="${itemIndex}"]`)
        ?.classList.contains('gallery-tile--selected') === true,
    index,
    { timeout: 5_000 }
  )

  const frameState = await finishFrameSampling(page)
  return {
    anchorStart: null,
    behaviorEquivalent: true,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 1,
    details: {
      inputSource: 'Playwright isolated Chromium touch pointer events',
      longPressSelectedItem: true,
      index
    }
  }
}

async function runBoundsScenario(page, imageContainer, sampleIndex) {
  const boundary = sampleIndex % 2 === 0 ? 'upper' : 'lower'
  await clickScrollbarAt(page, boundary === 'upper' ? 0 : 1)
  await page.waitForTimeout(1500)
  await imageContainer.hover()
  if (boundary === 'lower') {
    await page.mouse.wheel(0, config.deltaY * config.pulses * 4)
    await page.waitForTimeout(500)
  }
  const physicalAnchorStart = await imageContainer.evaluate((element) => element.scrollTop)
  const anchorStart = await beginFrameSampling(page)
  await page.waitForTimeout(100)
  await runWheelPulses(page, config.deltaY * (boundary === 'upper' ? -1 : 1))
  await waitForScrollTransactionCommit(page, physicalAnchorStart)
  const frameState = await finishFrameSampling(page)
  const frameMetrics = summarizeFrames(frameState, 0)
  const physicalAnchorEnd = await imageContainer.evaluate((element) => element.scrollTop)
  const physicalAnchorErrorPx = Math.abs(physicalAnchorEnd - physicalAnchorStart)

  return {
    anchorStart,
    behaviorEquivalent:
      // Chromium may suppress the first wheel event when the scroller is
      // already pinned to a hard boundary. Both boundaries must still receive
      // the rest of the gesture without any visual or physical drift.
      frameMetrics.wheelEventCount >= config.pulses - 1 &&
      frameMetrics.visualDisplacementErrorPx <= config.behaviorTolerancePx &&
      physicalAnchorErrorPx <= config.behaviorTolerancePx,
    frameMetrics,
    physicalAnchorErrorPx: round(physicalAnchorErrorPx),
    applyScrollBudget: true,
    interactionCount: config.pulses,
    details: { boundary }
  }
}

async function prepareWorkerDelayScenario(page) {
  await page.route('**/get/get-rows**', async (route) => {
    await new Promise((resolveDelay) => setTimeout(resolveDelay, config.workerDelayMs))
    await route.continue()
  })
  const initialStart = await firstRenderedRowStart(page)
  await clickScrollbarAt(page, 0.6)
  return { initialStart }
}

async function runWorkerDelayScenario(page, preparation) {
  if (preparation === null) {
    throw new Error('worker-delay scenario was not prepared before tracing')
  }
  const { initialStart } = preparation
  // The scrollbar gesture has its own scenario. This one isolates the delayed
  // worker response so a slow pointer event cannot be misattributed to row
  // materialization after the worker returns.
  await beginPassiveSampling(page)
  let finalStart = initialStart
  let changed = false
  try {
    finalStart = await waitForDifferentRow(page, initialStart, 10_000)
    changed = true
  } catch {
    finalStart = await firstRenderedRowStart(page)
  }
  await page.waitForTimeout(config.workerDelayMs + 500)
  const frameState = await finishFrameSampling(page)
  const rowCount = await renderedRowCount(page)

  return {
    anchorStart: null,
    behaviorEquivalent: changed && rowCount > 0,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 1,
    details: {
      initialStart,
      finalStart,
      rowCount,
      workerDelayMs: config.workerDelayMs,
      samplingPhase: 'delayed-row-return-after-scrollbar-input'
    }
  }
}

async function runScrollbarScenario(page) {
  const initialStart = await firstRenderedRowStart(page)
  await beginPassiveSampling(page)
  await clickScrollbarAt(page, 0.4)
  let clickedStart = initialStart
  try {
    clickedStart = await waitForDifferentRow(page, initialStart)
  } catch {
    clickedStart = await firstRenderedRowStart(page)
  }
  await dragScrollbar(page, 0.4, 0.75)
  await page.waitForTimeout(1500)
  const draggedStart = await firstRenderedRowStart(page)
  const frameState = await finishFrameSampling(page)
  const rowCount = await renderedRowCount(page)

  return {
    anchorStart: null,
    behaviorEquivalent:
      rowCount > 0 && clickedStart !== initialStart && draggedStart !== clickedStart,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 2,
    details: { initialStart, clickedStart, draggedStart, rowCount }
  }
}

function objectHashFromUrl(url) {
  const match = new URL(url).pathname.match(/\/object\/[^/]+\/[^/]+\/([^/.]+)\.[^/]+$/)
  return match?.[1]?.replace(/-v\d+$/, '') ?? null
}

async function runLocateScenario(page, requestedObjectUrls) {
  const locateHash = config.locate ?? requestedObjectUrls.map(objectHashFromUrl).find(Boolean)
  await beginPassiveSampling(page)
  let highlighted = false
  let queryRemoved = false

  if (locateHash) {
    const targetUrl = new URL(page.url())
    targetUrl.searchParams.set('locate', locateHash)
    await page.goto(targetUrl.toString(), { waitUntil: 'domcontentloaded' })
    await waitForGallery(page)
    highlighted = await page.locator('.locate-highlight').first().isVisible().catch(() => false)
    queryRemoved = !new URL(page.url()).searchParams.has('locate')
  }

  const frameState = await finishFrameSampling(page)
  const rowCount = await renderedRowCount(page)
  return {
    anchorStart: null,
    behaviorEquivalent: Boolean(locateHash) && highlighted && queryRemoved && rowCount > 0,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 1,
    details: { locateHash, highlighted, queryRemoved, rowCount }
  }
}

async function runResizeScenario(page) {
  const anchorItemIndex = await page
    .locator('[data-testid="gallery-item"]')
    .first()
    .getAttribute('data-item-index')
  await beginPassiveSampling(page)
  await page.setViewportSize({
    width: Math.max(640, config.viewportWidth - 320),
    height: Math.max(480, config.viewportHeight - 200)
  })
  await page.waitForTimeout(1500)
  const anchorStillRendered =
    anchorItemIndex !== null &&
    (await page.locator(`[data-item-index="${anchorItemIndex}"]`).count()) > 0
  const frameState = await finishFrameSampling(page)
  const rowCount = await renderedRowCount(page)

  return {
    anchorStart: null,
    behaviorEquivalent: anchorStillRendered && rowCount > 0,
    frameMetrics: summarizePassiveFrames(frameState),
    physicalAnchorErrorPx: 0,
    applyScrollBudget: false,
    interactionCount: 1,
    details: { anchorItemIndex, anchorStillRendered, rowCount }
  }
}

async function runScenario(
  page,
  imageContainer,
  sampleIndex,
  requestedObjectUrls,
  workerDelayPreparation,
  objectRouteState,
  thumbnailBurstPreparation
) {
  switch (config.scenario) {
    case 'discrete-wheel':
      return runDiscreteWheelScenario(page, imageContainer)
    case 'discrete-wheel-delay':
      return runDiscreteWheelScenario(page, imageContainer, true)
    case 'cdp-gesture':
      return runCdpScrollGestureScenario(page, imageContainer)
    case 'cdp-gesture-delay':
      return runCdpScrollGestureScenario(page, imageContainer, true)
    case 'cdp-discrete-gesture':
      return runCdpDiscreteGestureScenario(page, imageContainer)
    case 'thumbnail-throughput':
      return runThumbnailThroughputScenario(page, objectRouteState)
    case 'thumbnail-burst-during-scroll':
      return runThumbnailBurstDuringScrollScenario(
        page,
        imageContainer,
        objectRouteState,
        thumbnailBurstPreparation
      )
    case 'desktop-interaction':
      return runDesktopInteractionScenario(page)
    case 'mobile-interaction':
      return runMobileInteractionScenario(page)
    case 'native-wheel':
      return runNativeWheelScenario(page, imageContainer)
    case 'native-wheel-delay':
      return runNativeWheelScenario(page, imageContainer, true)
    case 'continuous-up':
      return runContinuousScenario(page, imageContainer, -1)
    case 'worker-delay':
      return runWorkerDelayScenario(page, workerDelayPreparation)
    case 'bounds':
      return runBoundsScenario(page, imageContainer, sampleIndex)
    case 'scrollbar':
      return runScrollbarScenario(page)
    case 'locate':
      return runLocateScenario(page, requestedObjectUrls)
    case 'resize':
      return runResizeScenario(page)
    case 'continuous-down':
    case 'mobile':
      return runContinuousScenario(page, imageContainer, 1)
    default:
      throw new Error(`unsupported scenario: ${config.scenario}`)
  }
}

async function runSample(browser, sampleIndex) {
  const mobile = config.scenario === 'mobile' || config.scenario === 'mobile-interaction'
  const objectRouteState = createObjectRouteState(
    config.scenario === 'thumbnail-throughput' ? config.thumbnailDelayMs : 0
  )
  const context = await browser.newContext({
    viewport: mobile
      ? { width: Math.min(config.viewportWidth, 390), height: Math.min(config.viewportHeight, 844) }
      : { width: config.viewportWidth, height: config.viewportHeight },
    deviceScaleFactor: 1,
    isMobile: mobile,
    hasTouch: mobile,
    ...(mobile
      ? {
          userAgent:
            'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 ' +
            'Chrome/149.0.0.0 Mobile Safari/537.36'
        }
      : {}),
    serviceWorkers: 'block'
  })
  // Keep font resolution identical when the baseline frontend is served from a
  // temporary Git-HEAD tree whose node_modules is linked into the workspace.
  await context.route('**/materialdesignicons-webfont.*', async (route) => {
    await route.fulfill({ status: 200, contentType: 'font/woff2', body: Buffer.alloc(0) })
  })
  await context.route('**/object/**', async (route) => {
    await objectRouteState.handle(route)
  })
  await installInstrumentation(context)
  const page = await context.newPage()
  const errors = []
  const requestedObjectUrls = []
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
  page.on('request', (request) => {
    if (new URL(request.url()).pathname.startsWith('/object/')) {
      requestedObjectUrls.push(request.url())
    }
  })

  try {
    const imageContainer = await loginAndWait(page)
    const workerDelayPreparation =
      config.scenario === 'worker-delay' ? await prepareWorkerDelayScenario(page) : null
    const thumbnailBurstPreparation =
      config.scenario === 'thumbnail-burst-during-scroll'
        ? await prepareThumbnailBurstScenario(page, objectRouteState)
        : null
    const stopTrace = await startTrace(page)
    const scenarioResult = await runScenario(
      page,
      imageContainer,
      sampleIndex,
      requestedObjectUrls,
      workerDelayPreparation,
      objectRouteState,
      thumbnailBurstPreparation
    )
    const galleryImageState = await readGalleryImageState(page)
    const traceEvents = await stopTrace()
    const frameMetrics = scenarioResult.frameMetrics
    const traceMetrics = summarizeTrace(traceEvents)
    const scrollEventWorkPerPulseMs = round(
      traceMetrics.scrollEventTotalMs / Math.max(scenarioResult.interactionCount, 1)
    )
    const scrollInteractionWorkPerPulseMs = round(
      (traceMetrics.scrollEventTotalMs + traceMetrics.scrollEndEventTotalMs) /
        Math.max(scenarioResult.interactionCount, 1)
    )
    const scrollInteractionWorkPerScrollEventMs = round(
      (traceMetrics.scrollEventTotalMs + traceMetrics.scrollEndEventTotalMs) /
        Math.max(traceMetrics.scrollEventCount, 1)
    )
    const scrollInteractionWorkPerEventMs = round(
      (traceMetrics.scrollEventTotalMs + traceMetrics.scrollEndEventTotalMs) /
        Math.max(traceMetrics.scrollEventCount + traceMetrics.scrollEndEventCount, 1)
    )
    const scrollBudgetWorkMs =
      scenarioResult.scrollBudgetUnit === 'scroll-event'
        ? scrollInteractionWorkPerEventMs
        : scrollInteractionWorkPerPulseMs
    const timerBudgetExceeded =
      config.scenario === 'worker-delay' &&
      traceMetrics.timerInstall0Count > config.timerZeroBudget
    const behaviorEquivalent = scenarioResult.behaviorEquivalent && !timerBudgetExceeded
    const enforceRunTaskBudget =
      config.scenario === 'cdp-gesture' || config.scenario === 'cdp-gesture-delay'
    const jankReasons = [
      ...(scenarioResult.applyScrollBudget &&
      scrollBudgetWorkMs >= config.scrollWorkPerPulseBudgetMs
        ? ['scroll-work-budget']
        : []),
      ...(frameMetrics.frameGapOver25MsCount > 0 ? ['frame-gap'] : []),
      ...(traceMetrics.updateLayoutTreeMaxMs >= 8 ? ['layout-budget'] : []),
      ...(enforceRunTaskBudget && traceMetrics.runTaskMaxMs >= 12
        ? ['run-task-budget']
        : []),
      ...(traceMetrics.longTaskCount > 0 ? ['long-task'] : []),
      ...(timerBudgetExceeded ? ['zero-timer-budget'] : [])
    ]
    const jankSignature = jankReasons.length > 0

    return {
      sample: sampleIndex + 1,
      scenario: config.scenario,
      anchorStart: scenarioResult.anchorStart,
      behaviorEquivalent,
      jankSignature,
      jankReasons,
      scrollEventWorkPerPulseMs,
      scrollInteractionWorkPerPulseMs,
      scrollInteractionWorkPerScrollEventMs,
      scrollInteractionWorkPerEventMs,
      physicalAnchorErrorPx: scenarioResult.physicalAnchorErrorPx,
      galleryImageState,
      scenarioDetails: scenarioResult.details,
      ...frameMetrics,
      ...traceMetrics,
      errors
    }
  } finally {
    objectRouteState.releaseHeld()
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
      p95: round(percentile(values, 0.95)),
      max: round(Math.max(...values))
    }
  }
  return {
    jankySamples: samples.filter((sample) => sample.jankSignature).length,
    behaviorEquivalentSamples: samples.filter((sample) => sample.behaviorEquivalent).length,
    metrics
  }
}

const browser = await chromium.launch({
  headless: !config.headed,
  ...(config.browser === 'chrome' ? { channel: 'chrome' } : {})
})
const browserVersion = browser.version()
let report
try {
  const samples = []
  for (let sampleIndex = 0; sampleIndex < config.samples; sampleIndex += 1) {
    const sample = await runSample(browser, sampleIndex)
    samples.push(sample)
    console.error(
      `sample ${sample.sample} (${sample.scenario}): dropped=${sample.droppedFrameCount} ` +
        `scrollInteractionWorkPerPulse=${sample.scrollInteractionWorkPerPulseMs}ms ` +
        `behaviorError=${sample.visualDisplacementErrorPx}px`
    )
  }
  report = {
    generatedAt: new Date().toISOString(),
    browserVersion,
    profileIsolation: 'Playwright temporary user-data-dir plus a fresh browser context per sample',
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
if (config.expect === 'strict-smooth' && report.aggregate.jankySamples > 0) {
  throw new Error(
    `expected every sample to be smooth, observed ` +
      `${report.aggregate.jankySamples}/${config.samples} jank signatures`
  )
}
