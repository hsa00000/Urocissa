import { useMessageStore } from '@/store/messageStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { useAlbumStore } from '@/store/albumStore'
import { IsolationId } from '@/type/types'
import axios from 'axios'
import { tryWithMessageStore } from '@/script/utils/try_catch'

export async function editAlbums(
  indexArray: number[],
  addAlbumsArray: string[],
  removeAlbumsArray: string[],
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const timestamp = prefetchStore.timestamp
  const messageStore = useMessageStore('mainId')
  const optimisticStore = useOptimisticStore(isolationId)
  const albumStore = useAlbumStore('mainId')

  if (timestamp === null) {
    messageStore.error('Cannot edit albums because timestamp is missing.')
    return
  }

  const payload = {
    indexSet: new Set(indexArray),
    addAlbumsArray: [...addAlbumsArray],
    removeAlbumsArray: [...removeAlbumsArray],
    timestamp: timestamp
  }

  // 1. 更新畫面上的資料 (Optimistic Update)
  optimisticStore.optimisticUpdateAlbums(payload, true)

  // 2. 更新全域相簿列表狀態 (雖目前無計數，但保持架構一致)
  if (addAlbumsArray.length > 0) {
    albumStore.optimisticAddAlbums(addAlbumsArray)
  }

  if (removeAlbumsArray.length > 0) {
    albumStore.optimisticRemoveAlbums(removeAlbumsArray)
  }

  // 3. 發送請求
  await tryWithMessageStore('mainId', async () => {
    const response = await axios.put('/put/edit_album', {
      indexArray,
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
