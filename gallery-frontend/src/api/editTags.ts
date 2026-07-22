import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { facetValueInfoSchema } from '@/type/schemas'
import { IsolationId, FacetValueInfo } from '@/type/types'
import axios from 'axios'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'
import { useSearchFacetStore } from '@/store/searchFacetStore'

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
  const searchFacetStore = useSearchFacetStore()
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
    const axiosResponse = await axios.put<FacetValueInfo[]>('/put/edit_tag', {
      selection,
      addTagsArray,
      removeTagsArray,
      timestamp
    })

    const tags = z.array(facetValueInfoSchema).parse(axiosResponse.data)
    searchFacetStore.applyTags(tags)

    messageStore.success('Successfully edited tags.')
  })
}
