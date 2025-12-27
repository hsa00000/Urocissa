import { useWorkerStore } from '@/store/workerStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { bindActionDispatch } from 'typesafe-agent-events'
import { toDataWorker } from '@/worker/workerApi'
import { FetchDataMethod, IsolationId } from '@type/types'
import { useTokenStore } from '@/store/tokenStore'

export async function fetchDataInWorker(
  fetchMethod: FetchDataMethod,
  batch: number,
  isolationId: IsolationId
) {
  const workerStore = useWorkerStore(isolationId)

  if (workerStore.worker === null) {
    workerStore.initializeWorker(isolationId)
  }
  const tokenStore = useTokenStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const dataWorker = workerStore.worker

  const postToWorker = bindActionDispatch(toDataWorker, (action) => {
    if (dataWorker) {
      dataWorker.postMessage(action)
    }
  })

  await tokenStore.refreshTimestampTokenIfExpired()

  const timestamp = prefetchStore.timestamp

  const timestampToken = tokenStore.timestampToken

  if (timestampToken === null) {
    console.error('timestampToken not found')
  } else if (timestamp !== null) {
    postToWorker.fetchData({
      fetchMethod: fetchMethod,
      batch: batch,
      timestamp: timestamp,
      timestampToken
    })
  }
}
