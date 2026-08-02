import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick, ref } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { useRouteResourceStore } from '@/store/routeResourceStore'
import { useResolvedRouteResource } from './useRouteResource'
import type { EnrichedUnifiedData } from '@/type/types'

vi.mock('@/store/workerStore', () => ({
  useWorkerStore: () => ({
    worker: null,
    imgWorker: [],
    initializeWorker: vi.fn(),
    terminateWorker: vi.fn()
  })
}))

function image(id: string, archived = false): EnrichedUnifiedData {
  return {
    type: 'image',
    id,
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
    isArchived: archived,
    isTrashed: false,
    updateAt: 0,
    timestamp: 1
  }
}

describe('route resource resolver', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('prefers an exact collection row and falls back to the direct snapshot', () => {
    const id = ref('a'.repeat(64))
    const main = useDataStore('mainId')
    const detail = useDataStore('detailId')
    detail.data.set(0, image(id.value, true))
    detail.hashMapData.set(id.value, 0)
    const resolved = useResolvedRouteResource(id, 'mainId', 'detailId')

    expect(resolved.resource.value).toMatchObject({ isolationId: 'detailId', index: 0 })

    main.data.set(7, image(id.value))
    main.hashMapData.set(id.value, 7)
    expect(resolved.resource.value).toMatchObject({ isolationId: 'mainId', index: 7 })
  })

  it('preserves rendered media while handing a direct resource to its collection row', async () => {
    const id = ref('c'.repeat(64))
    const main = useDataStore('mainId')
    const detail = useDataStore('detailId')
    const mainImages = useImgStore('mainId')
    const detailImages = useImgStore('detailId')

    detail.data.set(0, image(id.value))
    detail.hashMapData.set(id.value, 0)
    detailImages.imgOriginal.set(0, 'blob:direct-original')
    detailImages.imgUrl.set(0, 'blob:direct-thumbnail')

    const resolved = useResolvedRouteResource(id, 'mainId', 'detailId')
    expect(resolved.resource.value).toMatchObject({ isolationId: 'detailId', index: 0 })

    main.data.set(7, image(id.value))
    main.hashMapData.set(id.value, 7)
    await nextTick()

    expect(resolved.resource.value).toMatchObject({ isolationId: 'mainId', index: 7 })
    expect(mainImages.imgOriginal.get(7)).toBe('blob:direct-original')
    expect(mainImages.imgUrl.get(7)).toBe('blob:direct-thumbnail')
  })

  it('keeps the collection row when the direct snapshot finishes later', async () => {
    const id = ref('d'.repeat(64))
    const main = useDataStore('mainId')
    const detail = useDataStore('detailId')

    main.data.set(7, image(id.value))
    main.hashMapData.set(id.value, 7)
    const resolved = useResolvedRouteResource(id, 'mainId', 'detailId')

    expect(resolved.resource.value).toMatchObject({ isolationId: 'mainId', index: 7 })

    detail.data.set(0, image(id.value, true))
    detail.hashMapData.set(id.value, 0)
    await nextTick()

    expect(resolved.resource.value).toMatchObject({ isolationId: 'mainId', index: 7 })
  })

  it('surfaces direct 404 and wrong-type states without collection completion', () => {
    const id = ref('b'.repeat(64))
    const direct = useRouteResourceStore('subDetailId')
    direct.requestedId = id.value
    direct.status = 'not-found'
    const resolved = useResolvedRouteResource(id, 'subId', 'subDetailId', ['image', 'video'])

    expect(resolved.status.value).toBe('not-found')

    const detail = useDataStore('subDetailId')
    detail.data.set(0, {
      ...image(id.value),
      type: 'album',
      title: null,
      startTime: null,
      endTime: null,
      lastModifiedTime: 0,
      cover: null,
      itemCount: 0,
      itemSize: 0,
      shareList: {}
    })
    detail.hashMapData.set(id.value, 0)
    expect(resolved.status.value).toBe('wrong-type')
  })
})
