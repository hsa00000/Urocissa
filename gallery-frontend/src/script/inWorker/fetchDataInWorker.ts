import { useWorkerStore } from '@/store/workerStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { bindActionDispatch } from 'typesafe-agent-events'
import { toDataWorker } from '@/worker/workerApi'
import { FetchDataMethod, IsolationId } from '../common/types'
import { getTimestampToken } from '@/indexedDb/timestampToken'

export function fetchDataInWorker(
  fetchMethod: FetchDataMethod,
  batch: number,
  isolationId: IsolationId
) {
  const workerStore = useWorkerStore(isolationId)

  if (workerStore.worker === null) {
    workerStore.initializeWorker(isolationId)
  }

  const prefetchStore = usePrefetchStore(isolationId)
  const dataWorker = workerStore.worker

  const postToWorker = bindActionDispatch(toDataWorker, (action) => {
    if (dataWorker) {
      dataWorker.postMessage(action)
    }
  })
  const timestamp = prefetchStore.timestamp

  getTimestampToken()
    .then((timestampToken) => {
      if (timestampToken === null) {
        console.error('timestampToken not found')
        return
      }

      if (timestamp !== null) {
        // Photo data is fetched batch by batch
        postToWorker.fetchData({
          fetchMethod: fetchMethod,
          batch: batch,
          timestamp: timestamp,
          timestampToken
        })
      }
    })
    .catch((error: unknown) => {
      console.error('Failed to retrieve timestampToken:', error)
    })
}
