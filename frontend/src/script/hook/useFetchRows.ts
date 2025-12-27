// useFetchRows.ts
import { Ref, watch } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { fetchRowInWorker } from '@/api/fetchRow'
import debounce from 'lodash/debounce'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useRowStore } from '@/store/rowStore'
import { useOffsetStore } from '@/store/offsetStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { IsolationId } from '@type/types'

/**
 * Computes the sum of offsets for rows above the given scroll position.
 *
 * @param scrollTop - The given scroll position in pixels.
 * @returns The sum of offsets for all rows above the given scroll position.
 */
function computeOffSetSumOfAboveRowsIndex(scrollTop: number, isolationId: IsolationId) {
  const aboveRowsIndex: number[] = []
  const rowStore = useRowStore(isolationId)

  for (const row of rowStore.rowData.values()) {
    if (row.topPixelAccumulated + row.offset < scrollTop) {
      aboveRowsIndex.push(row.rowIndex)
    }
  }

  const offsetStore = useOffsetStore(isolationId)
  let offsetSum = 0

  aboveRowsIndex.forEach((rowIndex) => {
    const offset = offsetStore.offset.get(rowIndex)
    if (offset !== undefined) {
      offsetSum += offset
    } else {
      console.error('offset is undefined')
    }
  })

  return offsetSum
}

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
  startHeight: Ref<number>,
  endHeight: Ref<number>,
  isolationId: IsolationId,
  debounceTime = 50,
  maxWait = 100
) {
  const initializedStore = useInitializedStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const scrollTopStore = useScrollTopStore(isolationId)

  const debouncedFetch = debounce(
    async () => {
      if (initializedStore.initialized) {
        const offSetSumOfAboveRowsIndex = computeOffSetSumOfAboveRowsIndex(
          scrollTopStore.scrollTop,
          isolationId
        )
        const fixedHeight = 2400
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
      () => scrollTopStore.scrollTop,
      () => prefetchStore.updateFetchRowTrigger
    ],
    debouncedFetch,
    { immediate: true }
  )
}
