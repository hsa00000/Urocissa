import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useWorkerStore } from '@/store/workerStore'
import { IsolationId } from '@type/types'

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

  const timestamp = prefetchStore.timestamp
  if (timestamp !== null) {
    const payload = {
      indexSet: new Set(indexArray),
      addTagsArray: [...addTagsArray],
      removeTagsArray: [...removeTagsArray],
      timestamp: timestamp
    }
    const postToWorker = workerStore.postToWorker
    if (postToWorker !== undefined) {
      postToWorker.editTags(payload)
      optimisticUpdateTags.optimisticUpdateTags(payload, true)
    }
  }
}
