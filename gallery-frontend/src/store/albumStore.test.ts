import { beforeEach, describe, expect, it, vi } from 'vitest'
import axios from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { useAlbumStore } from './albumStore'

function album(albumId: string, albumName: string) {
  return { albumId, albumName, shareList: {} }
}

describe('album store refresh lifecycle', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('deduplicates forced refreshes and replaces stale album entries', async () => {
    const get = vi
      .spyOn(axios, 'get')
      .mockResolvedValueOnce({ status: 200, data: [album('old', 'Old album')] })
      .mockResolvedValueOnce({ status: 200, data: [album('new', 'New album')] })
    const store = useAlbumStore('mainId')

    await store.fetchAlbums()
    await Promise.all([store.refreshAlbums(), store.refreshAlbums()])

    expect(get).toHaveBeenCalledTimes(2)
    expect(store.albums.has('old')).toBe(false)
    expect(store.albums.get('new')).toMatchObject({
      albumName: 'New album',
      displayName: 'New album'
    })
  })
})
