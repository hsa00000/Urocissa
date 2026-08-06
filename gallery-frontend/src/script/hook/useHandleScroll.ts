import { getScrollUpperBound } from '@utils/getter'
import { IsolationId } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { throttle } from 'lodash'
import {
  computed,
  nextTick,
  readonly,
  shallowRef,
  type ComputedRef,
  type Ref
} from 'vue'

const scrollObservationInterval = 100
const fallbackScrollEndDelay = 200

export const hybridScrollDefaults = Object.freeze({
  edgeRangePx: 3_000,
  compensationAnchorPx: 200_000,
  compensationBufferPx: 600_000,
  positionTolerancePx: 1
})

export type HybridScrollMode = 'native-top' | 'compensated' | 'native-bottom'

export type ScrollSyncReason =
  | 'initialize'
  | 'locate'
  | 'scrollbar'
  | 'resize'
  | 'collection-reset'
  | 'external'

export interface HybridScrollOptions {
  edgeRangePx?: number
  compensationAnchorPx?: number
  compensationBufferPx?: number
  positionTolerancePx?: number
}

export interface HybridScrollDebugSnapshot {
  mode: HybridScrollMode
  physicalTop: number
  physicalMaximum: number
  projectionOrigin: number
  effectiveScrollTop: number
  logicalUpperBound: number
  totalHeight: number
  generation: number
  internalWriteCount: number
}

export interface HybridScrollController {
  readonly mode: Readonly<Ref<HybridScrollMode>>
  readonly effectiveScrollTop: Readonly<Ref<number>>
  readonly projectionOrigin: Readonly<Ref<number>>
  readonly logicalUpperBound: ComputedRef<number>
  readonly minimumBufferHeight: ComputedRef<number>
  readonly generation: Readonly<Ref<number>>
  readonly internalWriteCount: Readonly<Ref<number>>
  onScroll: () => void
  onScrollEnd: () => Promise<void>
  reconcileGeometry: (anchorShiftPx: number) => Promise<void>
  syncToLogicalPosition: (reason: ScrollSyncReason) => Promise<void>
  getDebugSnapshot: () => HybridScrollDebugSnapshot
  cancel: () => void
}

interface InternalPhysicalWrite {
  generation: number
  target: number
  externalMovementSerial: number
  reached: boolean
}

function clamp(value: number, lowerBound: number, upperBound: number): number {
  return Math.min(Math.max(value, lowerBound), upperBound)
}

export function selectHybridScrollMode(
  logicalTop: number,
  logicalUpperBound: number,
  _currentMode: HybridScrollMode,
  edgeRangePx: number = hybridScrollDefaults.edgeRangePx,
  tolerancePx: number = hybridScrollDefaults.positionTolerancePx
): HybridScrollMode {
  if (logicalUpperBound <= tolerancePx) return 'native-top'

  if (logicalUpperBound <= edgeRangePx * 2) {
    const midpoint = logicalUpperBound / 2
    return logicalTop <= midpoint ? 'native-top' : 'native-bottom'
  }

  if (logicalTop <= edgeRangePx) return 'native-top'
  if (logicalUpperBound - logicalTop <= edgeRangePx) return 'native-bottom'
  return 'compensated'
}

/**
 * Maps one logical virtual document onto three physical scrolling projections.
 *
 * `logical = physical - projectionOrigin` is the sole coordinate invariant. The Buffer
 * consumes the same projection origin, so a mode hand-off can move both the rendered rows and
 * physical scrollTop in one Vue render checkpoint without changing their viewport position.
 */
export function handleScroll(
  imageContainerRef: Ref<HTMLElement | null>,
  windowHeight: Ref<number>,
  isolationId: IsolationId,
  options: HybridScrollOptions = {}
): HybridScrollController {
  const edgeRangePx = options.edgeRangePx ?? hybridScrollDefaults.edgeRangePx
  const compensationAnchorPx =
    options.compensationAnchorPx ?? hybridScrollDefaults.compensationAnchorPx
  const compensationBufferPx =
    options.compensationBufferPx ?? hybridScrollDefaults.compensationBufferPx
  const positionTolerancePx =
    options.positionTolerancePx ?? hybridScrollDefaults.positionTolerancePx

  const scrollTopStore = useScrollTopStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const modeState = shallowRef<HybridScrollMode>('native-top')
  const effectiveScrollTopState = shallowRef(0)
  const projectionOriginState = shallowRef(0)
  const generationState = shallowRef(0)
  const internalWriteCountState = shallowRef(0)

  let fallbackScrollEndTimer: ReturnType<typeof setTimeout> | null = null
  let internalPhysicalWrite: InternalPhysicalWrite | null = null
  let externalMovementSerial = 0
  let lastObservedPhysicalTop = 0
  let operationQueue: Promise<void> = Promise.resolve()
  let modeSwitchQueued = false
  let geometryOperation: Promise<void> | null = null
  let pendingGeometryLogicalTop: number | null = null
  let geometryNotificationPending = false
  let authoritativeEpoch = 0
  let cancelled = false

  const isCancelled = (): boolean => cancelled
  const hasPendingGeometryNotification = (): boolean => geometryNotificationPending

  const logicalUpperBound = computed(() =>
    Math.max(getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value), 0)
  )

  const minimumBufferHeight = computed(() =>
    logicalUpperBound.value > edgeRangePx * 2 ? compensationBufferPx : 0
  )

  effectiveScrollTopState.value = clamp(
    scrollTopStore.scrollTop,
    0,
    logicalUpperBound.value
  )

  const getPhysicalMaximum = (imageContainer: HTMLElement): number =>
    Math.max(imageContainer.scrollHeight - imageContainer.clientHeight, 0)

  const readUnclampedLogicalTop = (imageContainer: HTMLElement): number =>
    imageContainer.scrollTop - projectionOriginState.value

  const readLogicalTop = (imageContainer: HTMLElement): number =>
    clamp(readUnclampedLogicalTop(imageContainer), 0, logicalUpperBound.value)

  const clearFallbackScrollEnd = () => {
    if (fallbackScrollEndTimer !== null) {
      clearTimeout(fallbackScrollEndTimer)
      fallbackScrollEndTimer = null
    }
  }

  const publishPhysicalPosition = () => {
    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return
    effectiveScrollTopState.value = readLogicalTop(imageContainer)
  }

  const throttledPublishPhysicalPosition = throttle(
    publishPhysicalPosition,
    scrollObservationInterval,
    { leading: true, trailing: true }
  )

  const enqueueOperation = <T>(operation: () => Promise<T>): Promise<T> => {
    const result = operationQueue.then(operation, operation)
    operationQueue = result.then(
      () => undefined,
      () => undefined
    )
    return result
  }

  const markTransition = (reason: string, nextMode: HybridScrollMode) => {
    if (typeof performance === 'undefined' || typeof performance.mark !== 'function') return
    performance.mark(`urocissa-scroll:${reason}:${nextMode}`)
  }

  const writePhysicalPosition = (
    imageContainer: HTMLElement,
    requestedTarget: number,
    generation: number
  ): boolean => {
    const target = clamp(requestedTarget, 0, getPhysicalMaximum(imageContainer))
    if (Math.abs(imageContainer.scrollTop - target) <= positionTolerancePx) {
      lastObservedPhysicalTop = imageContainer.scrollTop
      return false
    }

    internalPhysicalWrite = {
      generation,
      target,
      externalMovementSerial,
      reached: false
    }
    internalWriteCountState.value += 1
    imageContainer.scrollTop = target
    lastObservedPhysicalTop = imageContainer.scrollTop
    return true
  }

  const applyProjection = async (
    requestedLogicalTop: number,
    requestedMode: HybridScrollMode,
    reason: string
  ): Promise<void> => {
    const imageContainer = imageContainerRef.value
    if (imageContainer === null || cancelled) return
    const transactionEpoch = authoritativeEpoch

    clearFallbackScrollEnd()
    throttledPublishPhysicalPosition.cancel()

    let nextLogicalTop = clamp(requestedLogicalTop, 0, logicalUpperBound.value)
    let nextMode = selectHybridScrollMode(
      nextLogicalTop,
      logicalUpperBound.value,
      requestedMode,
      edgeRangePx,
      positionTolerancePx
    )

    generationState.value += 1
    const transactionGeneration = generationState.value

    for (let pass = 0; pass < 2; pass += 1) {
      const upperBound = logicalUpperBound.value
      nextLogicalTop = clamp(nextLogicalTop, 0, upperBound)
      nextMode = selectHybridScrollMode(
        nextLogicalTop,
        upperBound,
        nextMode,
        edgeRangePx,
        positionTolerancePx
      )

      modeState.value = nextMode
      scrollTopStore.scrollTop = nextLogicalTop
      effectiveScrollTopState.value = nextLogicalTop

      if (nextMode === 'native-top') {
        projectionOriginState.value = 0
      } else if (nextMode === 'compensated') {
        projectionOriginState.value = compensationAnchorPx - nextLogicalTop
      } else {
        projectionOriginState.value = getPhysicalMaximum(imageContainer) - upperBound
      }

      markTransition(reason, nextMode)
      await nextTick()

      if (transactionEpoch !== authoritativeEpoch || isCancelled()) return

      const updatedLogicalTop = clamp(nextLogicalTop, 0, logicalUpperBound.value)
      const updatedMode = selectHybridScrollMode(
        updatedLogicalTop,
        logicalUpperBound.value,
        nextMode,
        edgeRangePx,
        positionTolerancePx
      )
      if (updatedLogicalTop === nextLogicalTop && updatedMode === nextMode) break

      nextLogicalTop = updatedLogicalTop
      nextMode = updatedMode
    }

    if (
      imageContainerRef.value !== imageContainer ||
      transactionEpoch !== authoritativeEpoch ||
      isCancelled()
    ) {
      return
    }

    const finalUpperBound = logicalUpperBound.value
    nextLogicalTop = clamp(nextLogicalTop, 0, finalUpperBound)
    if (nextMode === 'native-bottom') {
      projectionOriginState.value = getPhysicalMaximum(imageContainer) - finalUpperBound
    }

    const targetPhysicalTop = nextLogicalTop + projectionOriginState.value
    writePhysicalPosition(imageContainer, targetPhysicalTop, transactionGeneration)
    effectiveScrollTopState.value = clamp(
      imageContainer.scrollTop - projectionOriginState.value,
      0,
      finalUpperBound
    )
  }

  const queueModeSwitch = () => {
    if (modeSwitchQueued || cancelled) return
    modeSwitchQueued = true
    const epoch = authoritativeEpoch

    void enqueueOperation(async () => {
      modeSwitchQueued = false
      if (cancelled || epoch !== authoritativeEpoch) return

      const imageContainer = imageContainerRef.value
      if (imageContainer === null) return

      const logicalTop = readLogicalTop(imageContainer)
      const desiredMode = selectHybridScrollMode(
        logicalTop,
        logicalUpperBound.value,
        modeState.value,
        edgeRangePx,
        positionTolerancePx
      )
      if (desiredMode !== modeState.value) {
        await applyProjection(logicalTop, desiredMode, 'threshold')
      }
    }).catch((error: unknown) => {
      console.error('Hybrid scroll mode switch failed:', error)
    })
  }

  const scheduleFallbackScrollEnd = (imageContainer: HTMLElement) => {
    if ('onscrollend' in imageContainer) return

    clearFallbackScrollEnd()
    fallbackScrollEndTimer = setTimeout(() => {
      fallbackScrollEndTimer = null
      void settleScrollTransaction()
    }, fallbackScrollEndDelay)
  }

  const onScroll = () => {
    const imageContainer = imageContainerRef.value
    if (imageContainer === null || cancelled) return

    const physicalTop = imageContainer.scrollTop
    if (
      internalPhysicalWrite !== null &&
      !internalPhysicalWrite.reached &&
      Math.abs(physicalTop - internalPhysicalWrite.target) <= positionTolerancePx
    ) {
      internalPhysicalWrite.reached = true
      lastObservedPhysicalTop = physicalTop
      effectiveScrollTopState.value = readLogicalTop(imageContainer)
      return
    }

    const physicalMovement = physicalTop - lastObservedPhysicalTop
    if (Math.abs(physicalMovement) <= positionTolerancePx) return

    externalMovementSerial += 1
    lastObservedPhysicalTop = physicalTop
    throttledPublishPhysicalPosition()
    scheduleFallbackScrollEnd(imageContainer)

    const logicalTop = readLogicalTop(imageContainer)
    const desiredMode = selectHybridScrollMode(
      logicalTop,
      logicalUpperBound.value,
      modeState.value,
      edgeRangePx,
      positionTolerancePx
    )
    if (desiredMode !== modeState.value) queueModeSwitch()
  }

  const consumeInternalScrollEnd = (): boolean => {
    const internalWrite = internalPhysicalWrite
    if (internalWrite?.reached !== true) return false

    internalPhysicalWrite = null
    return externalMovementSerial === internalWrite.externalMovementSerial
  }

  const settleScrollTransaction = async (): Promise<void> => {
    if (cancelled) return
    clearFallbackScrollEnd()
    throttledPublishPhysicalPosition.cancel()

    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return

    const logicalTop = readLogicalTop(imageContainer)
    const desiredMode = selectHybridScrollMode(
      logicalTop,
      logicalUpperBound.value,
      modeState.value,
      edgeRangePx,
      positionTolerancePx
    )

    if (desiredMode !== modeState.value || desiredMode === 'compensated') {
      await applyProjection(logicalTop, desiredMode, 'scroll-end')
      return
    }

    scrollTopStore.scrollTop = logicalTop
    effectiveScrollTopState.value = logicalTop
  }

  const onScrollEnd = (): Promise<void> => {
    if (consumeInternalScrollEnd()) return Promise.resolve()
    return enqueueOperation(settleScrollTransaction)
  }

  const syncToLogicalPosition = (reason: ScrollSyncReason): Promise<void> => {
    authoritativeEpoch += 1
    const epoch = authoritativeEpoch
    const requestedLogicalTop = scrollTopStore.scrollTop
    pendingGeometryLogicalTop = null
    geometryNotificationPending = false
    internalPhysicalWrite = null
    clearFallbackScrollEnd()
    throttledPublishPhysicalPosition.cancel()

    return enqueueOperation(async () => {
      if (cancelled || epoch !== authoritativeEpoch) return
      const logicalTop = clamp(requestedLogicalTop, 0, logicalUpperBound.value)
      const desiredMode = selectHybridScrollMode(
        logicalTop,
        logicalUpperBound.value,
        modeState.value,
        edgeRangePx,
        positionTolerancePx
      )
      await applyProjection(logicalTop, desiredMode, reason)
    })
  }

  const reconcileGeometry = (anchorShiftPx: number): Promise<void> => {
    const imageContainer = imageContainerRef.value
    pendingGeometryLogicalTop ??=
        geometryOperation === null && imageContainer !== null
          ? readUnclampedLogicalTop(imageContainer)
          : effectiveScrollTopState.value
    if (Number.isFinite(anchorShiftPx)) pendingGeometryLogicalTop += anchorShiftPx
    geometryNotificationPending = true
    if (geometryOperation !== null) return geometryOperation

    const epoch = authoritativeEpoch
    geometryOperation = enqueueOperation(async () => {
      do {
        if (cancelled || epoch !== authoritativeEpoch) {
          pendingGeometryLogicalTop = null
          geometryNotificationPending = false
          return
        }

        if (imageContainerRef.value === null) {
          pendingGeometryLogicalTop = null
          geometryNotificationPending = false
          return
        }

        const requestedLogicalTop =
          pendingGeometryLogicalTop ?? effectiveScrollTopState.value
        pendingGeometryLogicalTop = null
        geometryNotificationPending = false
        const targetLogicalTop = clamp(
          requestedLogicalTop,
          0,
          logicalUpperBound.value
        )
        const desiredMode = selectHybridScrollMode(
          targetLogicalTop,
          logicalUpperBound.value,
          modeState.value,
          edgeRangePx,
          positionTolerancePx
        )
        await applyProjection(targetLogicalTop, desiredMode, 'geometry')
      } while (hasPendingGeometryNotification())
    }).finally(() => {
      geometryOperation = null
    })

    return geometryOperation
  }

  const getDebugSnapshot = (): HybridScrollDebugSnapshot => {
    const imageContainer = imageContainerRef.value
    return {
      mode: modeState.value,
      physicalTop: imageContainer?.scrollTop ?? 0,
      physicalMaximum: imageContainer === null ? 0 : getPhysicalMaximum(imageContainer),
      projectionOrigin: projectionOriginState.value,
      effectiveScrollTop: effectiveScrollTopState.value,
      logicalUpperBound: logicalUpperBound.value,
      totalHeight: prefetchStore.totalHeight,
      generation: generationState.value,
      internalWriteCount: internalWriteCountState.value
    }
  }

  const cancel = () => {
    cancelled = true
    authoritativeEpoch += 1
    clearFallbackScrollEnd()
    throttledPublishPhysicalPosition.cancel()
    internalPhysicalWrite = null
    pendingGeometryLogicalTop = null
    geometryNotificationPending = false
  }

  return {
    mode: readonly(modeState),
    effectiveScrollTop: readonly(effectiveScrollTopState),
    projectionOrigin: readonly(projectionOriginState),
    logicalUpperBound,
    minimumBufferHeight,
    generation: readonly(generationState),
    internalWriteCount: readonly(internalWriteCountState),
    onScroll,
    onScrollEnd,
    reconcileGeometry,
    syncToLogicalPosition,
    getDebugSnapshot,
    cancel
  }
}
