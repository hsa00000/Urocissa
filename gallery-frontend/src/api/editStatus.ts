import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export interface EditStatusPayload {
  indexArray: number[]
  timestamp: number
  isFavorite?: boolean
  isArchived?: boolean
  isTrashed?: boolean
}

export async function editStatus(
  indexArray: number[],
  isolationId: IsolationId,
  options: {
    isFavorite?: boolean
    isArchived?: boolean
    isTrashed?: boolean
  }
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const optimisticStore = useOptimisticStore(isolationId)

  if (timestamp === null) {
    messageStore.error('Cannot edit status because timestamp is missing.')
    return
  }

  const payload: EditStatusPayload = {
    indexArray,
    timestamp,
    ...options
  }

  // Optimistic update
  optimisticStore.optimisticUpdateStatus(payload)

  await tryWithMessageStore('mainId', async () => {
    await axios.put('/put/edit_status', payload)

    messageStore.success('Successfully updated status.')
  })
}

export async function setFavorite(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editStatus(indexArray, isolationId, { isFavorite: value })
}

export async function setArchived(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editStatus(indexArray, isolationId, { isArchived: value })
}

export async function setTrashed(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editStatus(indexArray, isolationId, { isTrashed: value })
}
