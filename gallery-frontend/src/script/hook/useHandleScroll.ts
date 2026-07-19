import { getScrollUpperBound } from '@utils/getter'
import { IsolationId } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { throttle } from 'lodash'
import { Ref } from 'vue'
import { useConfigStore } from '@/store/configStore'

/**
 * Converts one cancelable pixel-mode wheel event into one instantaneous physical
 * buffer movement. This prevents Chrome's compositor from splitting a mouse-wheel
 * notch into partial scroll events; the existing scroll handler still performs
 * all virtual positioning, clamping, and physical-anchor compensation.
 */
export function applyWheelDeltaToPhysicalBuffer(
  imageContainer: HTMLElement | null,
  event: Pick<
    WheelEvent,
    'cancelable' | 'ctrlKey' | 'defaultPrevented' | 'deltaMode' | 'deltaY' | 'preventDefault'
  >
) {
  if (
    imageContainer === null ||
    !event.cancelable ||
    event.ctrlKey ||
    event.deltaMode !== 0 ||
    event.deltaY === 0
  ) {
    return false
  }

  event.preventDefault()
  if (!event.defaultPrevented) return false

  imageContainer.scrollTop += event.deltaY
  return true
}

/**
 * Throttled scroll handler for an image container that adjusts `scrollTop`, which is used to manage controlled scrolling.
 * This function compensates for changes in `imageContainerRef.value.scrollTop` caused by user scrolling,
 * ensuring the scroll position remains within `bufferHeight.value / 3`, as initialized in `initializeScrollPosition.ts`.
 *
 * @param imageContainerRef - Reference to the scrolling container element.
 * @param lastScrollTop - Reference to the last recorded scroll position.
 * @param scrollTop - Reference to the current scroll position.
 * @param mobile - Indicator for mobile device adjustments.
 * @param stopScroll - Flag to temporarily stop scrolling for mobile adjustments.
 * @param windowHeight - Reference to the window height for scroll limit calculations.
 *
 * @returns Throttled scroll event handler.
 */
export function handleScroll(
  imageContainerRef: Ref<HTMLElement | null>,
  lastScrollTop: Ref<number>,
  stopScroll: Ref<boolean>,
  windowHeight: Ref<number>,
  isolationId: IsolationId
) {
  const configStore = useConfigStore('mainId')
  const scrollTopStore = useScrollTopStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)

  const throttledHandleScroll = throttle(
    () => {
      if (imageContainerRef.value !== null) {
        const mobile = configStore.isMobile

        const difference = imageContainerRef.value.scrollTop - lastScrollTop.value

        if (prefetchStore.totalHeight - windowHeight.value < 0) {
          if (mobile) {
            stopScroll.value = true
            scrollTopStore.scrollTop = 0
            setTimeout(() => {
              stopScroll.value = false
            }, 100)
          } else {
            scrollTopStore.scrollTop = 0
          }
          imageContainerRef.value.scrollTop -= difference
          lastScrollTop.value = imageContainerRef.value.scrollTop
          return
        }

        const result = scrollTopStore.scrollTop + difference
        const upperBound = getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value)

        if (result < 0) {
          // If scrolling exceeds the lower bound, reset the scroll position to 0.
          if (mobile) {
            stopScroll.value = true
            scrollTopStore.scrollTop = 0
            setTimeout(() => {
              stopScroll.value = false
            }, 100)
          } else {
            scrollTopStore.scrollTop = 0
          }
        } else if (result >= upperBound) {
          // If scrolling exceeds the upper bound, reset the scroll position to the maximum allowed value.
          if (mobile) {
            stopScroll.value = true
            scrollTopStore.scrollTop = upperBound
            setTimeout(() => {
              stopScroll.value = false
            }, 100)
          } else {
            scrollTopStore.scrollTop = upperBound
          }
        } else {
          // Adjust the scroll position normally within the allowed range.
          scrollTopStore.scrollTop += difference
        }

        // Compensate for the change in scrollTop caused by the user's scroll action.
        imageContainerRef.value.scrollTop -= difference
        lastScrollTop.value = imageContainerRef.value.scrollTop
      }
    },
    100,
    {
      leading: true
    }
  )
  return throttledHandleScroll
}
