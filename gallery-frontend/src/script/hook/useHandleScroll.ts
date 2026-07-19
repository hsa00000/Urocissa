import { getScrollUpperBound } from '@utils/getter'
import { IsolationId } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { throttle } from 'lodash'
import { computed, nextTick, shallowRef, type ComputedRef, type Ref } from 'vue'
import { useConfigStore } from '@/store/configStore'

const scrollObservationInterval = 100
const fallbackScrollEndDelay = 200
const physicalPositionTolerance = 0.5

export interface CompensatedScrollController {
  /** Logical viewport position including Chrome's in-progress native physical movement. */
  readonly effectiveScrollTop: ComputedRef<number>
  /** Observe a native scroll without interrupting the browser's scrolling animation. */
  onScroll: () => void
  /** Commit the completed native movement to virtual scrollTop and restore the physical anchor. */
  onScrollEnd: () => Promise<void>
  /** Abandon an in-progress native transaction before locate, resize, or scrollbar jumps. */
  resetPhysicalAnchor: () => void
  /** Clear throttles and timers owned by the controller. */
  cancel: () => void
}

function clamp(value: number, lowerBound: number, upperBound: number): number {
  return Math.min(Math.max(value, lowerBound), upperBound)
}

/**
 * Keeps Chrome's native physical scrolling separate from the committed virtual position.
 *
 * During a native scroll transaction, the browser exclusively controls the element's
 * physical `scrollTop`. The resulting physical delta is exposed as `effectiveScrollTop`
 * for row selection and prefetching, while the committed Pinia value (and therefore the
 * Buffer transform) remains unchanged. Once `scrollend` fires, the final delta is committed
 * and the physical buffer is restored to its anchor in the same render checkpoint.
 */
export function handleScroll(
  imageContainerRef: Ref<HTMLElement | null>,
  lastScrollTop: Ref<number>,
  stopScroll: Ref<boolean>,
  windowHeight: Ref<number>,
  isolationId: IsolationId
): CompensatedScrollController {
  const configStore = useConfigStore('mainId')
  const scrollTopStore = useScrollTopStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const transientPhysicalDelta = shallowRef(0)

  let fallbackScrollEndTimer: ReturnType<typeof setTimeout> | null = null
  let mobileStopTimer: ReturnType<typeof setTimeout> | null = null
  let internalPhysicalTarget: number | null = null
  let committing = false

  const upperBound = computed(() =>
    Math.max(getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value), 0)
  )

  const effectiveScrollTop = computed(() =>
    clamp(scrollTopStore.scrollTop + transientPhysicalDelta.value, 0, upperBound.value)
  )

  const clearFallbackScrollEnd = () => {
    if (fallbackScrollEndTimer !== null) {
      clearTimeout(fallbackScrollEndTimer)
      fallbackScrollEndTimer = null
    }
  }

  const stopMobileScrollTemporarily = () => {
    if (!configStore.isMobile) return

    stopScroll.value = true
    if (mobileStopTimer !== null) {
      clearTimeout(mobileStopTimer)
    }
    mobileStopTimer = setTimeout(() => {
      stopScroll.value = false
      mobileStopTimer = null
    }, 100)
  }

  const observePhysicalDelta = () => {
    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return

    transientPhysicalDelta.value = imageContainer.scrollTop - lastScrollTop.value
  }

  const writePhysicalPosition = (imageContainer: HTMLElement, target: number) => {
    if (Math.abs(imageContainer.scrollTop - target) <= physicalPositionTolerance) return

    internalPhysicalTarget = target
    imageContainer.scrollTop = target
  }

  const throttledObservePhysicalDelta = throttle(
    observePhysicalDelta,
    scrollObservationInterval,
    { leading: true }
  )

  /**
   * Maps the logical content bounds onto the physical buffer for the current transaction.
   * Normal scrolling is never modified; a physical write happens only when the logical
   * viewport reaches an edge, where a native scroll container would stop as well.
   */
  const clampPhysicalPositionToLogicalBounds = (imageContainer: HTMLElement): boolean => {
    const anchor = lastScrollTop.value

    if (upperBound.value === 0) {
      transientPhysicalDelta.value = 0
      scrollTopStore.scrollTop = 0
      writePhysicalPosition(imageContainer, anchor)
      stopMobileScrollTemporarily()
      return true
    }

    const minimumPhysicalTop = anchor - scrollTopStore.scrollTop
    const maximumPhysicalTop = anchor + upperBound.value - scrollTopStore.scrollTop
    const clampedPhysicalTop = clamp(
      imageContainer.scrollTop,
      minimumPhysicalTop,
      maximumPhysicalTop
    )

    if (Math.abs(clampedPhysicalTop - imageContainer.scrollTop) <= physicalPositionTolerance) {
      return false
    }

    writePhysicalPosition(imageContainer, clampedPhysicalTop)
    transientPhysicalDelta.value = clampedPhysicalTop - anchor
    stopMobileScrollTemporarily()
    return true
  }

  const commitScrollTransaction = async () => {
    if (committing) return

    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return

    clearFallbackScrollEnd()
    throttledObservePhysicalDelta.cancel()

    const anchor = lastScrollTop.value
    const physicalDelta = imageContainer.scrollTop - anchor
    if (Math.abs(physicalDelta) <= physicalPositionTolerance) {
      transientPhysicalDelta.value = 0
      return
    }

    committing = true
    try {
      const nextScrollTop = clamp(
        scrollTopStore.scrollTop + physicalDelta,
        0,
        upperBound.value
      )

      // Vue batches these two writes, so consumers never observe committed + transient twice.
      scrollTopStore.scrollTop = nextScrollTop
      transientPhysicalDelta.value = 0

      // Apply the Buffer transform before restoring physical scrollTop. Both operations occur
      // before the browser's next paint, preserving the exact row position across the rebase.
      await nextTick()

      if (imageContainerRef.value === imageContainer) {
        writePhysicalPosition(imageContainer, anchor)
        lastScrollTop.value = anchor
      }
    } finally {
      committing = false
    }
  }

  const scheduleFallbackScrollEnd = (imageContainer: HTMLElement) => {
    if ('onscrollend' in imageContainer) return

    clearFallbackScrollEnd()
    fallbackScrollEndTimer = setTimeout(() => {
      fallbackScrollEndTimer = null
      void commitScrollTransaction()
    }, fallbackScrollEndDelay)
  }

  const onScroll = () => {
    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return

    if (internalPhysicalTarget !== null) {
      const reachedInternalTarget =
        Math.abs(imageContainer.scrollTop - internalPhysicalTarget) <= physicalPositionTolerance
      internalPhysicalTarget = null
      if (reachedInternalTarget) return
    }

    const reachedLogicalBoundary = clampPhysicalPositionToLogicalBounds(imageContainer)
    if (!reachedLogicalBoundary) {
      throttledObservePhysicalDelta()
    }
    scheduleFallbackScrollEnd(imageContainer)
  }

  const resetPhysicalAnchor = () => {
    clearFallbackScrollEnd()
    throttledObservePhysicalDelta.cancel()
    transientPhysicalDelta.value = 0

    const imageContainer = imageContainerRef.value
    if (imageContainer === null) return

    const anchor = lastScrollTop.value
    writePhysicalPosition(imageContainer, anchor)
  }

  const cancel = () => {
    clearFallbackScrollEnd()
    throttledObservePhysicalDelta.cancel()
    if (mobileStopTimer !== null) {
      clearTimeout(mobileStopTimer)
      mobileStopTimer = null
    }
  }

  return {
    effectiveScrollTop,
    onScroll,
    onScrollEnd: commitScrollTransaction,
    resetPhysicalAnchor,
    cancel
  }
}
