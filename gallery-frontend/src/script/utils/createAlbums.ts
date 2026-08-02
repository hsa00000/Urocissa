import axios from 'axios'
import { useMessageStore } from '@/store/messageStore'
import { useAlbumStore } from '@/store/albumStore'
import { GalleryAlbum, IsolationId } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { tryWithMessageStore } from './try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'
import { updateCachedResource } from './routeResourceCache'
import { useDataStore } from '@/store/dataStore'

export async function createNonEmptyAlbum(
  elementsIndex: SelectionInput,
  isolationId: IsolationId
): Promise<string | undefined> {
  const albumStore = useAlbumStore('mainId')
  const prefetchStore = usePrefetchStore(isolationId)

  return await tryWithMessageStore('mainId', async () => {
    const createNonEmptyAlbumData = {
      title: null,
      selection: normalizeSelection(elementsIndex),
      timestamp: prefetchStore.timestamp
    }

    const response = await axios.post<string>(
      '/post/create_non_empty_album',
      createNonEmptyAlbumData,
      {
        headers: {
          'Content-Type': 'application/json'
        }
      }
    )

    const messageStore = useMessageStore('mainId')
    messageStore.success('Album created successfully.')

    const newAlbumId = response.data
    await albumStore.refreshAlbums()
    return newAlbumId
  })
}

export async function createEmptyAlbum(): Promise<string | undefined> {
  const albumStore = useAlbumStore('mainId')

  return await tryWithMessageStore('mainId', async () => {
    const response = await axios.post<string>('/post/create_empty_album', {
      headers: {
        'Content-Type': 'application/json'
      }
    })

    const messageStore = useMessageStore('mainId')
    messageStore.success('Album created successfully.')

    const newAlbumId = response.data
    await albumStore.refreshAlbums()
    return newAlbumId
  })
}

export async function editTitle(
  album: GalleryAlbum,
  titleModelValue: string,
  isolationId: IsolationId = 'mainId'
) {
  const albumStore = useAlbumStore('mainId')

  if ((album.title ?? '') !== titleModelValue) {
    const id = album.id
    const title = titleModelValue === '' ? null : titleModelValue
    await axios.put('/put/set_album_title', {
      albumId: id,
      title: title
    })
    const albumInfo = albumStore.albums.get(id)

    if (albumInfo) {
      albumInfo.albumName = title
      albumInfo.displayName = albumInfo.albumName ?? 'Untitled'
    }
    const sourceStore = useDataStore(isolationId)
    const sourceIndex = sourceStore.hashMapData.get(id)
    const sourceData =
      sourceIndex === undefined ? undefined : sourceStore.data.get(sourceIndex)
    if (sourceData?.type === 'album') sourceData.title = title
    updateCachedResource(id, (data) => {
      if (data.type === 'album') data.title = title
    })
  }
}
