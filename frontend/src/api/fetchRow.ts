import { usePrefetchStore } from '@/store/prefetchStore'
import { useLocationStore } from '@/store/locationStore'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { toDataWorker } from '@/worker/workerApi'
import { clamp } from 'lodash'
import { bindActionDispatch } from 'typesafe-agent-events'
import { IsolationId } from '@type/types'
import { useTokenStore } from '@/store/tokenStore'
import { useConstStore } from '@/store/constStore'

/**
 * Fetches a row of data using a web worker if it isn't already queued.
 *
 * @param {number} index - The index of the row to fetch.
 */
export async function fetchRowInWorker(index: number, isolationId: IsolationId) {
  const prefetchStore = usePrefetchStore(isolationId)
  const locationStore = useLocationStore(isolationId)
  const queueStore = useQueueStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const constStore = useConstStore('mainId')
  if (prefetchStore.rowLength === 0) {
    return // No data to fetch
  }

  index = clamp(index, 0, prefetchStore.rowLength - 1)

  if (queueStore.row.has(index)) {
    return // Already fetched
  }

  if (locationStore.anchor !== null && locationStore.anchor !== index) {
    return // If a specific row is anchored, this make sure to fetch only that row
  }

  await tokenStore.refreshTimestampTokenIfExpired()

  const timestampToken = tokenStore.timestampToken

  if (timestampToken === null) {
    console.error('timestamp token not found')
    return
  }

  const workerStore = useWorkerStore(isolationId)

  if (workerStore.worker === null) {
    workerStore.initializeWorker(isolationId)
  }
  const dataWorker = workerStore.worker

  const postToWorker = bindActionDispatch(toDataWorker, (action) => {
    if (dataWorker) {
      dataWorker.postMessage(action)
    }
  })

  const timestamp = prefetchStore.timestamp

  if (timestamp !== null) {
    queueStore.row.add(index)
    postToWorker.fetchRow({
      index,
      timestamp,
      windowWidth: prefetchStore.windowWidth,
      isLastRow: index === prefetchStore.rowLength - 1,
      timestampToken,
      subRowHeightScale: constStore.subRowHeightScale
    })
  }
}
