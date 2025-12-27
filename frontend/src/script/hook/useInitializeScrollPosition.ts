import { watch, type Ref, type ComputedRef } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { fixedBigRowHeight, layoutBatchNumber } from '@/type/constants'
import { fetchRowInWorker } from '@/api/fetchRow'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { IsolationId } from '@type/types'

/**
 * Initializes scroll position and client height for the image container.
 * If `locateTo` is set, scrolls to the target row and fetches data.
 *
 * @param imageContainerRef - Image container element reference.
 * @param scrollTop - Current scroll position reference.
 * @param bufferHeight - Buffer height reference.
 * @param lastScrollTop - Last scroll position reference.
 * @param clientHeight - Client height reference.
 */
export function useInitializeScrollPosition(
  imageContainerRef: Ref<HTMLElement | null>,
  bufferHeight: ComputedRef<number>,
  lastScrollTop: Ref<number>,
  clientHeight: Ref<number>,
  windowWidth: Ref<number>,
  isolationId: IsolationId
): void {
  const initializedStore = useInitializedStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)

  watch(
    // Here windowWidth is watched for the case that when resizing,
    // the imageContainer.scrollTop may be reset to 0 (whenever bufferHeight becomes 0).
    [() => initializedStore.initialized, windowWidth],

    async () => {
      const scrollTopStore = useScrollTopStore(isolationId)
      const imageContainer = imageContainerRef.value
      if (imageContainer !== null && initializedStore.initialized) {
        imageContainer.scrollTop = bufferHeight.value / 3

        lastScrollTop.value = bufferHeight.value / 3

        clientHeight.value = imageContainer.clientHeight

        const jumpTo = prefetchStore.locateTo
        if (jumpTo !== null) {
          const targetRowIndex = Math.floor(jumpTo / layoutBatchNumber)
          scrollTopStore.scrollTop = targetRowIndex * fixedBigRowHeight
          await fetchRowInWorker(targetRowIndex, isolationId)
          prefetchStore.locateTo = null
        }
      }
    },
    { immediate: true, flush: 'post' }
  )
}
