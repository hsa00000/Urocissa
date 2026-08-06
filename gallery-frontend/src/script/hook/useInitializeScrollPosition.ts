import { watch, type Ref } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { fixedBigRowHeight, layoutBatchNumber } from '@/type/constants'
import { fetchRowInWorker } from '@/api/fetchRow'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { useLocationStore } from '@/store/locationStore'
import type { IsolationId } from '@type/types'

type InitialScrollSyncReason = 'initialize' | 'locate'

/**
 * Initializes the logical position only. Home's hybrid controller is the single owner of
 * projecting that position into the DOM, including both phases of a locate operation.
 */
export function useInitializeScrollPosition(
  imageContainerRef: Ref<HTMLElement | null>,
  clientHeight: Ref<number>,
  isolationId: IsolationId,
  syncToLogicalPosition: (reason: InitialScrollSyncReason) => Promise<void>
): void {
  const initializedStore = useInitializedStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const scrollTopStore = useScrollTopStore(isolationId)
  const locationStore = useLocationStore(isolationId)

  watch(
    () => initializedStore.initialized,
    async (initialized) => {
      const imageContainer = imageContainerRef.value
      if (imageContainer === null || !initialized) return

      clientHeight.value = imageContainer.clientHeight
      const jumpTo = prefetchStore.locateTo
      if (jumpTo === null) {
        await syncToLogicalPosition('initialize')
        return
      }

      const targetRowIndex = Math.floor(jumpTo / layoutBatchNumber)
      locationStore.locationIndex = jumpTo
      locationStore.anchor = targetRowIndex
      locationStore.pendingLocateTarget = jumpTo
      scrollTopStore.scrollTop = targetRowIndex * fixedBigRowHeight

      // First phase: move directly from the current projection to the coarse row target.
      await syncToLogicalPosition('locate')
      await fetchRowInWorker(targetRowIndex, isolationId)
      prefetchStore.locateTo = null
    },
    { immediate: true, flush: 'post' }
  )
}
