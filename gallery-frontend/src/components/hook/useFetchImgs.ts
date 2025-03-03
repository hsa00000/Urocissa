import { computed, Ref, watch } from 'vue'
import { fetchDataInWorker } from '@/script/inWorker/fetchDataInWorker'
import { useDataStore } from '@/store/dataStore'
import debounce from 'lodash/debounce'
import { getArrayValue } from '@utils/getter'
import { IsolationId } from '@/script/common/types'
/**
 * Hook to fetch image batches for visible rows in a virtual scroll.
 *
 * @param visibleRows - Reactive reference to visible row ranges.
 * @param visibleRowsLength - Reactive reference to the length of visible rows.
 * @param batchNumber - Number of items per batch.
 * @param debounceTime - Debounce delay in milliseconds.
 * @param maxWait - Max wait time for the debounced function.
 */
export function useFetchImgs(
  visibleRows: Ref<{ start: number; end: number }[]>,
  visibleRowsLength: Ref<number>,
  batchNumber: number,
  isolationId: IsolationId,
  debounceTime = 75,
  maxWait = 1000
) {
  const debouncedFetch = debounce(
    () => {
      const dataStore = useDataStore(isolationId)
      const length = visibleRowsLength.value
      if (length > 0) {
        const startBatchIndex = Math.max(
          Math.floor(getArrayValue(visibleRows.value, 0).start / batchNumber) - 1,
          0
        )
        const endBatchIndex =
          Math.floor(getArrayValue(visibleRows.value, length - 1).end / batchNumber) + 1

        for (let batchIndex = startBatchIndex; batchIndex <= endBatchIndex; batchIndex++) {
          if (dataStore.batchFetched.get(batchIndex) !== true) {
            fetchDataInWorker('batch', batchIndex, isolationId)
          }
        }
      }
    },
    debounceTime,
    { maxWait }
  )

  /* Computes `visibleRowsId` from `visibleRows` to detect changes in the visible range and triggers
  a debounced fetch to load image batches only when necessary. */

  const visibleRowsId = computed(() => {
    const length = visibleRows.value.length
    if (length > 0) {
      const start = getArrayValue(visibleRows.value, 0).start
      const end = getArrayValue(visibleRows.value, length - 1).end
      return `${start}-${end}`
    } else {
      return ''
    }
  })

  watch(visibleRowsId, debouncedFetch, { immediate: true })
}
