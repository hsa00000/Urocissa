import { getScrollUpperBound } from '@/script/common/functions'
import { IsolationId } from '@/script/common/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { throttle } from 'lodash'
import { Ref } from 'vue'

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
  mobile: string | null,
  stopScroll: Ref<boolean>,
  windowHeight: Ref<number>,
  isolationId: IsolationId
) {
  const throttledHandleScroll = throttle(
    () => {
      if (imageContainerRef.value !== null) {
        const scrollTopStore = useScrollTopStore(isolationId)
        const prefetchStore = usePrefetchStore(isolationId)
        const difference = imageContainerRef.value.scrollTop - lastScrollTop.value
        const result = scrollTopStore.scrollTop + difference

        if (result < 0) {
          // If scrolling exceeds the lower bound, reset the scroll position to 0.
          if (mobile !== null) {
            stopScroll.value = true
            scrollTopStore.scrollTop = 0
            setTimeout(() => {
              stopScroll.value = false
            }, 100)
          } else {
            scrollTopStore.scrollTop = 0
          }
        } else if (result >= getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value)) {
          // If scrolling exceeds the upper bound, reset the scroll position to the maximum allowed value.
          if (mobile !== null) {
            stopScroll.value = true
            scrollTopStore.scrollTop = getScrollUpperBound(
              prefetchStore.totalHeight,
              windowHeight.value
            )
            setTimeout(() => {
              stopScroll.value = false
            }, 100)
          } else {
            scrollTopStore.scrollTop = getScrollUpperBound(
              prefetchStore.totalHeight,
              windowHeight.value
            )
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
