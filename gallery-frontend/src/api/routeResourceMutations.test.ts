import { beforeEach, describe, expect, it, vi } from 'vitest'
import axios from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { editAlbums } from './editAlbums'
import { editFlags } from './editFlags'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import type { EnrichedUnifiedData, IsolationId } from '@/type/types'

vi.mock('@/store/routeResourceStore', () => ({
  useRouteResourceStore: () => ({ requestedId: null, clear: vi.fn() })
}))

const ID = 'a'.repeat(64)

function image(): EnrichedUnifiedData {
  return {
    type: 'image',
    id: ID,
    width: 10,
    height: 10,
    ext: 'jpg',
    size: 10,
    tags: [],
    exif: {},
    phash: null,
    thumbhash: null,
    thumbhashUrl: null,
    cacheVersion: 0,
    pending: false,
    albums: [],
    alias: [],
    description: null,
    isFavorite: false,
    isArchived: false,
    isTrashed: false,
    updateAt: 0,
    timestamp: 1
  }
}

function seed(isolationId: IsolationId): void {
  const store = useDataStore(isolationId)
  store.data.set(0, image())
  store.hashMapData.set(ID, 0)
}

function memberships(isolationId: IsolationId): string[] {
  const data = useDataStore(isolationId).data.get(0)
  if (data === undefined || data.type === 'album') throw new Error('expected cached media')
  return data.albums
}

describe('route resource mutation cache synchronization', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
    seed('mainId')
    seed('detailId')
    usePrefetchStore('mainId').timestamp = 10
  })

  it('synchronizes flags only after the backend accepts the mutation', async () => {
    const putSpy = vi.spyOn(axios, 'put').mockRejectedValueOnce(new Error('network failed'))
    await editFlags([0], { isArchived: true }, 'mainId')

    expect(useDataStore('detailId').data.get(0)?.isArchived).toBe(false)

    putSpy.mockResolvedValueOnce({ status: 200 })
    await editFlags([0], { isArchived: true }, 'mainId')

    expect(useDataStore('detailId').data.get(0)?.isArchived).toBe(true)
  })

  it('updates album memberships in every cached copy after success', async () => {
    vi.spyOn(axios, 'put').mockResolvedValue({ status: 200 })

    await editAlbums([0], ['album-a'], [], 'mainId')

    expect(memberships('mainId')).toEqual(['album-a'])
    expect(memberships('detailId')).toEqual(['album-a'])
  })
})
