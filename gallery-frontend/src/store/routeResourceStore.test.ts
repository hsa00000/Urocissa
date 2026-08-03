import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { fetchRouteResource } from '@/api/fetchRouteResource'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useRouteResourceStore } from './routeResourceStore'
import type { RouteResourceSnapshot } from '@/type/types'

vi.mock('@/api/fetchRouteResource', () => ({
  fetchRouteResource: vi.fn()
}))

const workerStoreMock = vi.hoisted(() => ({
  worker: null as Worker | null,
  imgWorker: [] as Worker[],
  initializeWorker: vi.fn(),
  terminateWorker: vi.fn()
}))

vi.mock('@/store/workerStore', () => ({
  useWorkerStore: () => workerStoreMock
}))

function snapshot(id: string, timestamp: number): RouteResourceSnapshot {
  return {
    prefetch: { timestamp, dataLength: 1, locateTo: 0 },
    token: `snapshot-token-${timestamp}`,
    data: {
      abstractData: {
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
        cacheVersion: 0,
        pending: false,
        albums: [],
        alias: [],
        description: null,
        isFavorite: false,
        isArchived: true,
        isTrashed: false,
        updateAt: 0
      },
      timestamp,
      token: `hash-token-${timestamp}`
    }
  }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  let reject!: (reason?: unknown) => void
  const promise = new Promise<T>((resolvePromise, rejectPromise) => {
    resolve = resolvePromise
    reject = rejectPromise
  })
  return { promise, resolve, reject }
}

describe('route resource store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    workerStoreMock.worker = null
    workerStoreMock.imgWorker = []
    workerStoreMock.initializeWorker.mockImplementation(() => {
      workerStoreMock.worker = {} as Worker
      workerStoreMock.imgWorker = [{} as Worker]
    })
    workerStoreMock.terminateWorker.mockImplementation(() => {
      workerStoreMock.worker = null
      workerStoreMock.imgWorker = []
    })
  })

  it('hydrates a one-item snapshot at index zero and deduplicates in-flight loads', async () => {
    const pending = deferred<RouteResourceSnapshot>()
    vi.mocked(fetchRouteResource).mockReturnValue(pending.promise)
    const store = useRouteResourceStore('detailId')

    const first = store.load('a'.repeat(64))
    const second = store.load('a'.repeat(64))
    expect(fetchRouteResource).toHaveBeenCalledOnce()

    pending.resolve(snapshot('a'.repeat(64), 11))
    await Promise.all([first, second])

    expect(store.status).toBe('ready')
    expect(useDataStore('detailId').hashMapData.get('a'.repeat(64))).toBe(0)
    expect(useDataStore('detailId').data.get(0)?.isArchived).toBe(true)
    expect(usePrefetchStore('detailId')).toMatchObject({
      timestamp: 11,
      dataLength: 1,
      locateTo: 0,
      locateResolution: { requestedId: 'a'.repeat(64), index: 0 }
    })
  })

  it('does not let a stale response overwrite a newer route parameter', async () => {
    const firstPending = deferred<RouteResourceSnapshot>()
    const secondPending = deferred<RouteResourceSnapshot>()
    vi.mocked(fetchRouteResource)
      .mockReturnValueOnce(firstPending.promise)
      .mockReturnValueOnce(secondPending.promise)
    const store = useRouteResourceStore('detailId')

    const first = store.load('a'.repeat(64))
    const second = store.load('b'.repeat(64))
    secondPending.resolve(snapshot('b'.repeat(64), 22))
    await second
    firstPending.resolve(snapshot('a'.repeat(64), 11))
    await first

    expect(store.requestedId).toBe('b'.repeat(64))
    expect(store.status).toBe('ready')
    expect(useDataStore('detailId').hashMapData.has('a'.repeat(64))).toBe(false)
    expect(useDataStore('detailId').hashMapData.get('b'.repeat(64))).toBe(0)
  })

  it('terminates the previous image generation before reusing index zero', async () => {
    vi.mocked(fetchRouteResource)
      .mockResolvedValueOnce(snapshot('a'.repeat(64), 11))
      .mockResolvedValueOnce(snapshot('b'.repeat(64), 22))
    const store = useRouteResourceStore('detailId')

    await store.load('a'.repeat(64))
    expect(workerStoreMock.initializeWorker).toHaveBeenCalledOnce()

    await store.load('b'.repeat(64))

    expect(workerStoreMock.terminateWorker).toHaveBeenCalledOnce()
    expect(workerStoreMock.initializeWorker).toHaveBeenCalledTimes(2)
    expect(useDataStore('detailId').hashMapData.has('a'.repeat(64))).toBe(false)
    expect(useDataStore('detailId').hashMapData.get('b'.repeat(64))).toBe(0)
  })

  it('exposes a direct 404 instead of waiting for a collection', async () => {
    vi.mocked(fetchRouteResource).mockRejectedValue(
      Object.assign(new Error('Request failed'), {
        isAxiosError: true,
        response: {
          status: 404,
          data: { kind: 'NotFound', message: 'Item not found' }
        }
      })
    )
    const store = useRouteResourceStore('detailId')

    await store.load('c'.repeat(64))

    expect(store.status).toBe('not-found')
    expect(store.errorMessage).toBe('Item not found')
    expect(useDataStore('detailId').data.size).toBe(0)
  })
})
