import { useMessageStore } from '@/store/messageStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'
import {
  selectedCachedResourceIds,
  updateCachedResource
} from '@/script/utils/routeResourceCache'
export async function editAlbums(
  selectionInput: SelectionInput,
  addAlbumsArray: string[],
  removeAlbumsArray: string[],
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const selection = normalizeSelection(selectionInput)
  const selectedIds = selectedCachedResourceIds(isolationId, selection)

  if (timestamp === null) {
    messageStore.error('Cannot edit albums because timestamp is missing.')
    return
  }

  const succeeded = await tryWithMessageStore('mainId', async () => {
    const response = await axios.put('/put/edit_album', {
      selection,
      addAlbumsArray,
      removeAlbumsArray,
      timestamp
    })

    if (response.status === 200) {
      messageStore.success('Successfully edited albums.')
      return true
    } else {
      messageStore.error(`Failed to edit albums. Server responded with status ${response.status}.`)
      return false
    }
  })

  if (succeeded !== true) return
  for (const resourceId of selectedIds) {
    updateCachedResource(resourceId, (data) => {
      if (data.type === 'album') return
      for (const albumId of addAlbumsArray) {
        if (!data.albums.includes(albumId)) data.albums.push(albumId)
      }
      data.albums = data.albums.filter((albumId) => !removeAlbumsArray.includes(albumId))
    })
  }
}
