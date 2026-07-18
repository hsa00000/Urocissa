import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { tagInfoSchema } from '@/type/schemas'
import { IsolationId, TagInfo } from '@/type/types'
import axios from 'axios'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'

export async function editTags(
  selectionInput: SelectionInput,
  addTagsArray: string[],
  removeTagsArray: string[],
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const optimisticStore = useOptimisticStore(isolationId)
  const selection = normalizeSelection(selectionInput)

  if (timestamp === null) {
    messageStore.error('Cannot edit tags because timestamp is missing.')
    return
  }

  const payload = {
    selection,
    addTagsArray: [...addTagsArray],
    removeTagsArray: [...removeTagsArray],
    timestamp: timestamp
  }
  optimisticStore.optimisticUpdateTags(payload, true)

  await tryWithMessageStore('mainId', async () => {
    const axiosResponse = await axios.put<TagInfo[]>('/put/edit_tag', {
      selection,
      addTagsArray,
      removeTagsArray,
      timestamp
    })

    const tagsArraySchema = z.array(tagInfoSchema)
    tagsArraySchema.parse(axiosResponse.data)

    messageStore.success('Successfully edited tags.')
  })
}
