// useFetchRows.ts
import { onScopeDispose, watch } from 'vue'
import type { Ref } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { fetchRowInWorker } from '@/api/fetchRow'
import debounce from 'lodash/debounce'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useRowStore } from '@/store/rowStore'
import { useOffsetStore } from '@/store/offsetStore'
import type { IsolationId } from '@type/types'
import { fixedBigRowHeight } from '@/type/constants'
import { computeOffSetSumOfAboveRowsIndex } from '@utils/rowOffset'

/**
 * Custom hook to fetch rows of data in a virtual scrolling environment based on the current scroll position.
 *
 * @param scrollTop - Reference to the current scroll position.
 * @param startHeight - Reference to the start height of the viewport.
 * @param endHeight - Reference to the end height of the viewport.
 * @param debounceTime - Time in milliseconds to debounce fetch requests (default: 50ms).
 * @param maxWait - Maximum wait time in milliseconds for debounced requests (default: 100ms).
 */
export function useFetchRows(
  startHeight: Readonly<Ref<number>>,
  endHeight: Readonly<Ref<number>>,
  isolationId: IsolationId,
  debounceTime = 50,
  maxWait = 100
) {
  const initializedStore = useInitializedStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const rowStore = useRowStore(isolationId)
  const offsetStore = useOffsetStore(isolationId)

  const debouncedFetch = debounce(
    async () => {
      if (initializedStore.initialized) {
        const offSetSumOfAboveRowsIndex = computeOffSetSumOfAboveRowsIndex(
          startHeight.value,
          rowStore.rowData,
          offsetStore.offset
        )
        const fixedHeight = fixedBigRowHeight
        const startHeightOffseted = startHeight.value - offSetSumOfAboveRowsIndex - fixedHeight
        const endHeightOffseted = endHeight.value - offSetSumOfAboveRowsIndex + fixedHeight
        const startIndex = Math.floor(startHeightOffseted / fixedHeight)
        const endIndex = Math.ceil(endHeightOffseted / fixedHeight)

        for (let i = startIndex; i <= endIndex; i++) {
          await fetchRowInWorker(i, isolationId)
        }

        const prependBatch = Math.floor(startHeightOffseted / fixedHeight) - 1

        await fetchRowInWorker(prependBatch, isolationId)

        const appendBatch = Math.ceil(endHeightOffseted / fixedHeight) + 1

        await fetchRowInWorker(appendBatch, isolationId)
      }
    },
    debounceTime,
    { maxWait }
  )

  watch(
    [
      () => initializedStore.initialized,
      startHeight,
      () => prefetchStore.updateFetchRowTrigger
    ],
    debouncedFetch,
    { immediate: true }
  )

  onScopeDispose(() => {
    debouncedFetch.cancel()
  })
}
