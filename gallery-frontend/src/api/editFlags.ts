import { useMessageStore } from '@/store/messageStore'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { createSelectionMatcher, normalizeSelection } from '@/type/selection'

export interface EditFlagsPayload {
  selection: ReturnType<typeof normalizeSelection>
  timestamp: number
  isFavorite?: boolean
  isArchived?: boolean
  isTrashed?: boolean
}

/**
 * Update boolean flags (isFavorite, isArchived, isTrashed) on one or more items.
 *
 * This is the dedicated API for flag mutations, separate from `editTags` which
 * handles string tags. The edit-tags modals (EditTagsModal / EditBatchTagsModal)
 * surface these flags as virtual "flag items" in the combobox alongside real tags,
 * then split the result at submit time: real tags → editTags, flags → editFlags.
 */
export async function editFlags(
  selectionInput: SelectionInput,
  flags: { isFavorite?: boolean; isArchived?: boolean; isTrashed?: boolean },
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const dataStore = useDataStore(isolationId)
  const selection = normalizeSelection(selectionInput)

  if (timestamp === null) {
    messageStore.error('Cannot edit flags because timestamp is missing.')
    return
  }

  // Optimistic update
  const isSelected = createSelectionMatcher(selection)
  for (const [index, data] of dataStore.data) {
    if (isSelected(index)) {
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
      selection,
      timestamp,
      ...flags
    })

    messageStore.success('Successfully updated.')
  })
}

// Convenience functions
export async function setFavorite(selection: SelectionInput, value: boolean, isolationId: IsolationId) {
  await editFlags(selection, { isFavorite: value }, isolationId)
}

export async function setArchived(selection: SelectionInput, value: boolean, isolationId: IsolationId) {
  await editFlags(selection, { isArchived: value }, isolationId)
}

export async function setTrashed(selection: SelectionInput, value: boolean, isolationId: IsolationId) {
  await editFlags(selection, { isTrashed: value }, isolationId)
}
