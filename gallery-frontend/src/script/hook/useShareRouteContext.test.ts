import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { effectScope, nextTick, reactive } from 'vue'
import type { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import { useShareStore } from '@/store/shareStore'
import { useShareRouteContext } from './useShareRouteContext'
import type { ShareInfo } from '@/db/db'

const dbMocks = vi.hoisted(() => ({
  getShareInfo: vi.fn(),
  storeShareInfo: vi.fn().mockResolvedValue(undefined),
  clearShareInfo: vi.fn().mockResolvedValue(undefined)
}))

vi.mock('@/db/db', () => dbMocks)

function shareInfo(albumId: string, shareId: string, password: string | null): ShareInfo {
  return { albumId, shareId, password }
}

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

async function flushAsyncWork(): Promise<void> {
  await nextTick()
  await Promise.resolve()
  await Promise.resolve()
  await nextTick()
}

describe('share route context', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
    dbMocks.storeShareInfo.mockResolvedValue(undefined)
  })

  it('hydrates credentials before mounting the collection and clears headers on leave', async () => {
    dbMocks.getShareInfo.mockResolvedValue(
      shareInfo('album-a', 'share-a', 'stored-password')
    )
    const route = reactive({
      params: { albumId: 'album-a', shareId: 'share-a' },
      query: {},
      meta: { baseName: 'share', level: 1 }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const scope = effectScope()
    const context = scope.run(() => useShareRouteContext(route))
    expect(context).toBeDefined()
    expect(context?.basicString.value).toBeUndefined()

    await flushAsyncWork()

    const store = useShareStore('mainId')
    expect(store).toMatchObject({
      albumId: 'album-a',
      shareId: 'share-a',
      password: 'stored-password'
    })
    expect(context?.basicString.value).toBe(
      'and(trashed:false, album:"album-a")'
    )
    expect(dbMocks.storeShareInfo).toHaveBeenCalledWith(
      shareInfo('album-a', 'share-a', 'stored-password')
    )

    store.password = 'updated-password'
    await flushAsyncWork()
    expect(dbMocks.storeShareInfo).toHaveBeenLastCalledWith(
      shareInfo('album-a', 'share-a', 'updated-password')
    )

    scope.stop()
    expect(store.albumId).toBeNull()
    expect(store.shareId).toBeNull()
    expect(store.password).toBeNull()
    expect(context?.basicString.value).toBeUndefined()
    expect(dbMocks.clearShareInfo).not.toHaveBeenCalled()
  })

  it('does not let stale credentials overwrite a newer share route', async () => {
    const oldRequest = deferred<ShareInfo | null>()
    dbMocks.getShareInfo
      .mockReturnValueOnce(oldRequest.promise)
      .mockResolvedValueOnce(shareInfo('album-b', 'share-b', 'new-password'))
    const route = reactive({
      params: { albumId: 'album-a', shareId: 'share-a' },
      query: {},
      meta: { baseName: 'share', level: 1 }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const scope = effectScope()
    const context = scope.run(() => useShareRouteContext(route))

    await nextTick()
    route.params.albumId = 'album-b'
    route.params.shareId = 'share-b'
    await flushAsyncWork()

    oldRequest.resolve(shareInfo('album-a', 'share-a', 'old-password'))
    await flushAsyncWork()

    expect(useShareStore('mainId')).toMatchObject({
      albumId: 'album-b',
      shareId: 'share-b',
      password: 'new-password'
    })
    expect(context?.basicString.value).toBe(
      'and(trashed:false, album:"album-b")'
    )
    expect(dbMocks.storeShareInfo).not.toHaveBeenCalledWith(
      shareInfo('album-a', 'share-a', 'old-password')
    )

    scope.stop()
  })
})
