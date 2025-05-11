import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { tagInfoSchema } from '@/type/schemas'
import { IsolationId, TagInfo } from '@/type/types'
import axios from 'axios'
import { z } from 'zod'

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

  try {
    const axiosResponse = await axios.put<TagInfo[]>('/put/edit_tag', {
      indexArray,
      addTagsArray,
      removeTagsArray,
      timestamp
    })

    const tagsArraySchema = z.array(tagInfoSchema)
    tagsArraySchema.parse(axiosResponse.data)

    messageStore.success('Successfully edited tags.')
  } catch (error) {
    messageStore.error('Failed to edit tags due to a network or validation error.')
    console.error(error)
  }
}
