import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useWorkerStore } from '@/store/workerStore'
import { toDataWorker } from '@/worker/workerApi'
import { bindActionDispatch } from 'typesafe-agent-events'
import { IsolationId } from '../common/types'

export function editTagsInWorker(
  indexArray: number[],
  addTagsArray: string[],
  removeTagsArray: string[],
  isolationId: IsolationId
) {
  const workerStore = useWorkerStore('mainId')
  const prefetchStore = usePrefetchStore(isolationId)
  const optimisticUpdateTags = useOptimisticStore(isolationId)
  if (workerStore.worker === null) {
    workerStore.initializeWorker('mainId')
  }

  const dataWorker = workerStore.worker
  const postToWorker = bindActionDispatch(toDataWorker, (action) => {
    if (dataWorker) {
      dataWorker.postMessage(action)
    }
  })
  const timestamp = prefetchStore.timestamp
  if (timestamp !== null) {
    const payload = {
      indexSet: new Set(indexArray),
      addTagsArray: [...addTagsArray],
      removeTagsArray: [...removeTagsArray],
      timestamp: timestamp
    }
    postToWorker.editTags(payload)
    optimisticUpdateTags.optimisticUpdateTags(payload, true)
  }
}
