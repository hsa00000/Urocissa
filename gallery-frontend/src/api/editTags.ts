import { useMessageStore } from '@/store/messageStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { facetValueInfoSchema } from '@/type/schemas'
import { IsolationId, FacetValueInfo } from '@/type/types'
import axios from 'axios'
import { z } from 'zod'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { createSelectionMatcher, normalizeSelection } from '@/type/selection'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useDataStore } from '@/store/dataStore'
import {
  selectedCachedResourceIds,
  updateCachedResource
} from '@/script/utils/routeResourceCache'

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
  const dataStore = useDataStore(isolationId)
  const selection = normalizeSelection(selectionInput)
  const selectedIds = selectedCachedResourceIds(isolationId, selection)

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
  const selected = createSelectionMatcher(selection)
  const previousTags = new Map<number, string[]>()
  for (const [index, data] of dataStore.data) {
    if (selected(index)) previousTags.set(index, [...data.tags])
  }
  const previousFacets = searchFacetStore.tags.map((tag) => ({ ...tag }))
  optimisticStore.optimisticUpdateTags(payload, true)

  const result = await tryWithMessageStore('mainId', async () => {
    const axiosResponse = await axios.put<FacetValueInfo[]>('/put/edit_tag', {
      selection,
      addTagsArray,
      removeTagsArray,
      timestamp
    })

    for (const resourceId of selectedIds) {
      updateCachedResource(resourceId, (_data, cachedIsolationId, index) => {
        if (cachedIsolationId === isolationId && selected(index)) return
        const dataStore = useDataStore(cachedIsolationId)
        dataStore.addTags(index, addTagsArray)
        dataStore.removeTags(index, removeTagsArray)
      })
    }

    const parsedTags = z.array(facetValueInfoSchema).safeParse(axiosResponse.data)
    if (parsedTags.success) {
      searchFacetStore.applyTags(parsedTags.data)
    } else {
      searchFacetStore.clearAll()
      void searchFacetStore.fetchFacets()
      messageStore.error('Tags were updated, but the refreshed tag list was invalid. Reloading it.')
      return { accepted: true }
    }

    messageStore.success('Successfully edited tags.')
    return { accepted: true }
  })

  if (result?.accepted === true) return
  for (const [index, tags] of previousTags) {
    const data = dataStore.data.get(index)
    if (data !== undefined) data.tags = tags
  }
  searchFacetStore.applyTags(previousFacets)
}
