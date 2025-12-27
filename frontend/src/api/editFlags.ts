import { useMessageStore } from '@/store/messageStore'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export interface EditFlagsPayload {
  indexArray: number[]
  timestamp: number
  isFavorite?: boolean
  isArchived?: boolean
  isTrashed?: boolean
}

export async function editFlags(
  indexArray: number[],
  flags: { isFavorite?: boolean; isArchived?: boolean; isTrashed?: boolean },
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const dataStore = useDataStore(isolationId)

  if (timestamp === null) {
    messageStore.error('Cannot edit flags because timestamp is missing.')
    return
  }

  // Optimistic update
  for (const index of indexArray) {
    const data = dataStore.data.get(index)
    if (data) {
      if (flags.isFavorite !== undefined) {
        data.isFavorite = flags.isFavorite
      }
      if (flags.isArchived !== undefined) {
        data.isArchived = flags.isArchived
      }
      if (flags.isTrashed !== undefined) {
        data.isTrashed = flags.isTrashed
      }
    }
  }

  await tryWithMessageStore('mainId', async () => {
    await axios.put('/put/edit_flags', {
      indexArray,
      timestamp,
      ...flags
    })

    messageStore.success('Successfully updated.')
  })
}

// Convenience functions
export async function setFavorite(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editFlags(indexArray, { isFavorite: value }, isolationId)
}

export async function setArchived(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editFlags(indexArray, { isArchived: value }, isolationId)
}

export async function setTrashed(indexArray: number[], value: boolean, isolationId: IsolationId) {
  await editFlags(indexArray, { isTrashed: value }, isolationId)
}
