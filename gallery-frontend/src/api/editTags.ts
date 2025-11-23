import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export async function editTags(
  indexArray: number[],
  addTagsArray: string[],
  removeTagsArray: string[],
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const optimisticStore = useOptimisticStore(isolationId)

  if (timestamp === null) {
    messageStore.error('Cannot edit tags because timestamp is missing.')
    return
  }

  const payload = {
    indexSet: new Set(indexArray),
    addTagsArray: [...addTagsArray],
    removeTagsArray: [...removeTagsArray],
    timestamp: timestamp
  }
  optimisticStore.optimisticUpdateTags(payload, true)

  await tryWithMessageStore('mainId', async () => {
    await axios.put('/put/edit_tag', {
      indexArray,
      addTagsArray,
      removeTagsArray,
      timestamp
    })

    messageStore.success('Successfully edited tags.')
  })
}
