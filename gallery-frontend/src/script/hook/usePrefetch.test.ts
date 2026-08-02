import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { effectScope, reactive, ref } from 'vue'
import type { EffectScope } from 'vue'
import type { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import type { PrefetchReturn } from '@/type/types'
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

function snapshot(timestamp: number): PrefetchReturn {
  return {
    prefetch: { timestamp, dataLength: 1, locateTo: null },
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
    const beforeApply = vi.fn()
    prefetchMock.mockResolvedValueOnce(snapshot(1)).mockResolvedValueOnce(snapshot(2))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { beforeApply })
    })
    await vi.advanceTimersByTimeAsync(100)

    route.query.sort = 'random'
    await vi.advanceTimersByTimeAsync(100)

    expect(prefetchMock).toHaveBeenNthCalledWith(1, 'old-filter', '', 'descending', null)
    expect(prefetchMock).toHaveBeenNthCalledWith(2, 'old-filter', '', 'random', null)
    expect(beforeApply).toHaveBeenCalledTimes(2)
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
    const beforeApply = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'mainId', { beforeApply })
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
    expect(beforeApply).toHaveBeenCalledTimes(1)
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
    const beforeApply = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { beforeApply })
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
    expect(beforeApply).toHaveBeenCalledTimes(1)
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
    const beforeApply = vi.fn()
    prefetchMock.mockResolvedValue(snapshot(1))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', {
        beforeApply,
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
    expect(beforeApply).toHaveBeenCalledTimes(5)
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
    const beforeApply = vi.fn()
    prefetchMock.mockReturnValueOnce(oldRequest.promise).mockResolvedValueOnce(snapshot(2))

    scope.run(() => {
      usePrefetch(filter, windowWidth, route, 'subId', { beforeApply })
    })
    await vi.advanceTimersByTimeAsync(100)

    filter.value = 'new-filter'
    await vi.advanceTimersByTimeAsync(100)
    expect(usePrefetchStore('subId').timestamp).toBe(2)

    oldRequest.resolve(snapshot(1))
    await Promise.resolve()
    await Promise.resolve()

    expect(prefetchMock).toHaveBeenNthCalledWith(2, 'new-filter', '', 'descending', null)
    expect(beforeApply).toHaveBeenCalledTimes(1)
    expect(usePrefetchStore('subId').timestamp).toBe(2)
    expect(useTokenStore('subId').timestampToken).toBe('token-2')
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
  })
})
