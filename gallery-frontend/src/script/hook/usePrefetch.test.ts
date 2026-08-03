import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { effectScope, reactive, ref } from 'vue'
import type { EffectScope } from 'vue'
import type { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import type { PrefetchReturn } from '@/type/types'
import { useInitializedStore } from '@/store/initializedStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useTokenStore } from '@/store/tokenStore'
import { usePrefetch } from './usePrefetch'

const prefetchMock = vi.hoisted(() => vi.fn())
const fetchScrollbarMock = vi.hoisted(() => vi.fn().mockResolvedValue(undefined))

vi.mock('@/api/fetchPrefetch', () => ({ prefetch: prefetchMock }))
vi.mock('@/api/fetchScrollbar', () => ({ fetchScrollbar: fetchScrollbarMock }))
vi.mock('@/store/configStore', () => ({
  useConfigStore: () => ({ fetchConfig: vi.fn().mockResolvedValue(undefined) })
}))
vi.mock('@/store/searchFacetStore', () => ({
  useSearchFacetStore: () => ({ fetched: true, fetchFacets: vi.fn() })
}))
vi.mock('@/store/albumStore', () => ({
  useAlbumStore: () => ({ fetched: true, fetchAlbums: vi.fn() })
}))
vi.mock('@/route/initialRouteLocate', () => ({
  consumeInitialMainLocateOverride: () => null
}))

function snapshot(timestamp: number, locateTo: number | null = null): PrefetchReturn {
  return {
    prefetch: { timestamp, dataLength: 1, locateTo },
    token: `token-${timestamp}`,
    resolvedShare: null
  }
}

function deferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
} {
  let resolvePromise!: (value: T) => void
  const promise = new Promise<T>((resolve) => {
    resolvePromise = resolve
  })
  return { promise, resolve: resolvePromise }
}

describe('reactive collection prefetch', () => {
  let scope: EffectScope

  beforeEach(() => {
    vi.useFakeTimers()
    setActivePinia(createPinia())
    prefetchMock.mockReset()
    fetchScrollbarMock.mockClear()
    scope = effectScope()
  })

  afterEach(() => {
    scope.stop()
    vi.useRealTimers()
  })

  it('reloads query changes without disposing the scope', async () => {
    const filter = ref<string | null>('old-filter')
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params: {},
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const onRequestStart = vi.fn()
    prefetchMock.mockResolvedValueOnce(snapshot(1)).mockResolvedValueOnce(snapshot(2))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { onRequestStart })
    })
    await vi.advanceTimersByTimeAsync(100)

    route.query.sort = 'random'
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenNthCalledWith(1, 'old-filter', '', 'descending', null)
    expect(prefetchMock).toHaveBeenNthCalledWith(2, 'old-filter', '', 'random', null)
    expect(onRequestStart).toHaveBeenCalledTimes(2)
    expect(usePrefetchStore('subId').timestamp).toBe(2)
    expect(useTokenStore('subId').timestampToken).toBe('token-2')
  })

  it('does not reload mainId when navigation only changes Level 1 and Level 2', async () => {
    const filter = ref<string | null>(null)
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const params: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params,
      meta: { level: 1, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const onRequestStart = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'mainId', { onRequestStart })
    })
    await vi.advanceTimersByTimeAsync(100)

    route.params.hash = 'a'.repeat(64)
    route.meta.level = 2
    await vi.advanceTimersByTimeAsync(100)

    delete route.params.hash
    route.meta.level = 1
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenCalledTimes(1)
    expect(prefetchMock).toHaveBeenCalledWith(null, '', 'descending', null)
    expect(onRequestStart).toHaveBeenCalledTimes(1)
  })

  it('does not reload subId when navigation only changes Level 3 and Level 4', async () => {
    const filter = ref<string | null>('album-filter')
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const params: Record<string, string | undefined> = { hash: 'album-id' }
    const route = reactive({
      query,
      params,
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const onRequestStart = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { onRequestStart })
    })
    await vi.advanceTimersByTimeAsync(100)

    route.params.subhash = 'b'.repeat(64)
    route.meta.level = 4
    await vi.advanceTimersByTimeAsync(100)

    delete route.params.subhash
    route.meta.level = 3
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenCalledTimes(1)
    expect(prefetchMock).toHaveBeenCalledWith(
      'album-filter',
      '',
      'descending',
      null
    )
    expect(onRequestStart).toHaveBeenCalledTimes(1)
  })

  it('reloads actual filter, locate, priority, and reload-trigger changes', async () => {
    const filter = ref<string | null>('old-filter')
    const windowWidth = ref(1200)
    const reloadTrigger = ref(false)
    const query: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params: { hash: 'album-id' },
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const onRequestStart = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', {
        onRequestStart,
        reloadTrigger
      })
    })
    await vi.advanceTimersByTimeAsync(100)

    filter.value = 'new-filter'
    await vi.advanceTimersByTimeAsync(100)

    route.query.locate = 'child-id'
    await vi.advanceTimersByTimeAsync(100)

    route.query.priority_id = 'priority-id'
    await vi.advanceTimersByTimeAsync(100)

    reloadTrigger.value = true
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenNthCalledWith(1, 'old-filter', '', 'descending', null)
    expect(prefetchMock).toHaveBeenNthCalledWith(2, 'new-filter', '', 'descending', null)
    expect(prefetchMock).toHaveBeenNthCalledWith(
      3,
      'new-filter',
      '',
      'descending',
      'child-id'
    )
    expect(prefetchMock).toHaveBeenNthCalledWith(
      4,
      'new-filter',
      'priority-id',
      'descending',
      'child-id'
    )
    expect(prefetchMock).toHaveBeenNthCalledWith(
      5,
      'new-filter',
      'priority-id',
      'descending',
      'child-id'
    )
    expect(onRequestStart).toHaveBeenCalledTimes(5)
  })

  it('does not let a stale response replace the latest query snapshot', async () => {
    const oldRequest = deferred<PrefetchReturn>()
    const filter = ref<string | null>('old-filter')
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params: {},
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const onRequestStart = vi.fn()
    prefetchMock.mockReturnValueOnce(oldRequest.promise).mockResolvedValueOnce(snapshot(2))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { onRequestStart })
    })
    await vi.advanceTimersByTimeAsync(100)

    filter.value = 'new-filter'
    await vi.advanceTimersByTimeAsync(100)
    expect(usePrefetchStore('subId').timestamp).toBe(2)

    oldRequest.resolve(snapshot(1))
    await Promise.resolve()
    await Promise.resolve()

    expect(prefetchMock).toHaveBeenNthCalledWith(2, 'new-filter', '', 'descending', null)
    expect(onRequestStart).toHaveBeenCalledTimes(2)
    expect(usePrefetchStore('subId').timestamp).toBe(2)
    expect(useTokenStore('subId').timestampToken).toBe('token-2')
  })

  it('invalidates the visible snapshot when a throttled query request starts', async () => {
    const throttledRequest = deferred<PrefetchReturn>()
    const filter = ref<string | null>('old-filter')
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params: {},
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    const initializedStore = useInitializedStore('subId')
    const prefetchStore = usePrefetchStore('subId')
    const resetVisibleSnapshot = vi.fn(() => {
      initializedStore.initialized = false
      prefetchStore.timestamp = null
      prefetchStore.calculateLength(0)
    })
    prefetchMock
      .mockResolvedValueOnce(snapshot(1))
      .mockReturnValueOnce(throttledRequest.promise)

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', {
        onRequestStart: resetVisibleSnapshot
      })
    })
    await vi.advanceTimersByTimeAsync(100)
    expect(initializedStore.initialized).toBe(true)
    expect(prefetchStore.timestamp).toBe(1)
    expect(prefetchStore.dataLength).toBe(1)
    prefetchStore.locateResolution = { requestedId: 'old-locate', index: null }
    resetVisibleSnapshot.mockClear()

    filter.value = 'new-filter'
    await vi.advanceTimersByTimeAsync(74)

    expect(resetVisibleSnapshot).not.toHaveBeenCalled()
    expect(initializedStore.initialized).toBe(true)

    await vi.advanceTimersByTimeAsync(1)

    expect(prefetchMock).toHaveBeenNthCalledWith(
      2,
      'new-filter',
      '',
      'descending',
      null
    )
    expect(resetVisibleSnapshot).toHaveBeenCalledOnce()
    expect(prefetchStore.locateResolution).toBeNull()
    expect(initializedStore.initialized).toBe(false)
    expect(prefetchStore.timestamp).toBeNull()
    expect(prefetchStore.dataLength).toBe(0)

    await vi.advanceTimersByTimeAsync(500)
    expect(initializedStore.initialized).toBe(false)
    expect(prefetchStore.dataLength).toBe(0)

    throttledRequest.resolve(snapshot(2))
    await Promise.resolve()
    await Promise.resolve()

    expect(resetVisibleSnapshot).toHaveBeenCalledOnce()
    expect(initializedStore.initialized).toBe(true)
    expect(prefetchStore.timestamp).toBe(2)
    expect(prefetchStore.dataLength).toBe(1)
  })

  it('does not apply a Level 3 sort query to the background Level 1 collection', async () => {
    const filter = ref<string | null>(null)
    const windowWidth = ref(1200)
    const query: Record<string, string | undefined> = {}
    const route = reactive({
      query,
      params: { hash: 'album-id' },
      meta: { level: 3, baseName: 'home' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'mainId')
    })
    await vi.advanceTimersByTimeAsync(100)

    route.query.sort = 'random'
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenCalledTimes(1)
    expect(prefetchMock).toHaveBeenCalledWith(
      null,
      '',
      'descending',
      'album-id'
    )
    expect(usePrefetchStore('mainId').locateResolution).toEqual({
      requestedId: 'album-id',
      index: null
    })
  })

  it('records the share route ID whose collection membership was resolved', async () => {
    const id = '7'.repeat(64)
    const filter = ref<string | null>('share-album-filter')
    const windowWidth = ref(1200)
    const route = reactive({
      query: {},
      params: { hash: id },
      meta: { level: 2, baseName: 'share' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    prefetchMock.mockResolvedValue(snapshot(3))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'mainId')
    })
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenCalledWith(
      'share-album-filter',
      '',
      'descending',
      id
    )
    expect(usePrefetchStore('mainId')).toMatchObject({
      timestamp: 3,
      locateTo: null,
      locateResolution: { requestedId: id, index: null }
    })
  })

  it('keeps a successful locate resolution after the jump cursor is consumed', async () => {
    const id = '8'.repeat(64)
    const filter = ref<string | null>('share-album-filter')
    const windowWidth = ref(1200)
    const route = reactive({
      query: {},
      params: { hash: id },
      meta: { level: 2, baseName: 'share' }
    }) as unknown as RouteLocationNormalizedLoadedGeneric
    prefetchMock.mockResolvedValue(snapshot(4, 0))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'mainId')
    })
    await vi.advanceTimersByTimeAsync(100)

    const prefetchStore = usePrefetchStore('mainId')
    expect(prefetchStore.locateResolution).toEqual({ requestedId: id, index: 0 })

    prefetchStore.locateTo = null

    expect(prefetchStore.locateResolution).toEqual({ requestedId: id, index: 0 })
  })
})
