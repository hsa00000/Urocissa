import { useMessageStore } from '@/store/messageStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'
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

  if (timestamp === null) {
    messageStore.error('Cannot edit albums because timestamp is missing.')
    return
  }

  await tryWithMessageStore('mainId', async () => {
    const response = await axios.put('/put/edit_album', {
      selection,
      addAlbumsArray,
      removeAlbumsArray,
      timestamp
    })

    if (response.status === 200) {
      messageStore.success('Successfully edited albums.')
    } else {
      messageStore.error(`Failed to edit albums. Server responded with status ${response.status}.`)
    }
  })
}
