import axios from 'axios'
import { useMessageStore } from '@/store/messageStore'
import { useAlbumStore } from '@/store/albumStore'
import { GalleryAlbum, IsolationId } from '@type/types'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { tryWithMessageStore } from './try_catch'

export async function createNonEmptyAlbum(
  elementsIndex: number[],
  isolationId: IsolationId
): Promise<string | undefined> {
  const albumStore = useAlbumStore('mainId')
  const prefetchStore = usePrefetchStore(isolationId)

  return await tryWithMessageStore('mainId', async () => {
    const createNonEmptyAlbumData = {
      title: null,
      elementsIndex: elementsIndex,
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
    await albumStore.fetchAlbums()
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
    await albumStore.fetchAlbums()
    return newAlbumId
  })
}

export async function editTitle(album: GalleryAlbum, titleModelValue: string) {
  const albumStore = useAlbumStore('mainId')
  const dataStore = useDataStore('mainId')

  if ((album.title ?? '') !== titleModelValue) {
    const id = album.id
    const title = titleModelValue === '' ? null : titleModelValue
    await axios.put('/put/set_album_title', {
      albumId: id,
      title: title
    })
    const albumInfo = albumStore.albums.get(id)

    const index = dataStore.hashMapData.get(album.id)
    if (index !== undefined) {
      const data = dataStore.data.get(index)

      if (albumInfo && data && data.type === 'album') {
        albumInfo.albumName = title
        albumInfo.displayName = albumInfo.albumName ?? 'Untitled'
        data.title = title
      } else {
        console.error(`Cannot find album with id ${id}`)
      }
    }
  }
}
