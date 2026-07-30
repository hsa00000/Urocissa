import { createPinia, setActivePinia } from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  createSavedSearch,
  deleteSavedSearch,
  fetchSavedSearches,
  renameSavedSearch,
  reorderSavedSearches
} from '@/api/savedSearches'
import type { SavedSearch } from '@/type/types'
import { useSavedSearchStore } from './savedSearchStore'

vi.mock('@/api/savedSearches', () => ({
  createSavedSearch: vi.fn(),
  deleteSavedSearch: vi.fn(),
  fetchSavedSearches: vi.fn(),
  renameSavedSearch: vi.fn(),
  reorderSavedSearches: vi.fn()
}))

const first: SavedSearch = {
  id: '020b6f4f-5c28-4f8c-81f8-bc22949f1ee8',
  name: 'Family',
  context: 'favorite',
  query: 'tag:family',
  sortOrder: 'descending'
}

const second: SavedSearch = {
  id: '99ee3b9a-7bf2-461f-abeb-1ab4b33fe697',
  name: 'Recent videos',
  context: 'videos',
  query: 'after:2026-01-01',
  sortOrder: 'ascending'
}

function deferred<T>(): {
  promise: Promise<T>
  resolve: (value: T) => void
} {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

describe('saved search store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.clearAllMocks()
  })

  it('loads at most once for the lifetime of the page', async () => {
    const pendingLoad = deferred<SavedSearch[]>()
    vi.mocked(fetchSavedSearches).mockReturnValue(pendingLoad.promise)
    const store = useSavedSearchStore()

    const firstLoad = store.loadOnce()
    const remountedLoad = store.loadOnce()
    expect(fetchSavedSearches).toHaveBeenCalledOnce()
    pendingLoad.resolve([first])

    await expect(firstLoad).resolves.toBe(true)
    await expect(remountedLoad).resolves.toBe(true)

    expect(fetchSavedSearches).toHaveBeenCalledOnce()
    expect(store.searches).toEqual([first])
    expect(store.loadAttempted).toBe(true)
    expect(store.loaded).toBe(true)
  })

  it('does not retry an unsuccessful initial load during remounts', async () => {
    vi.mocked(fetchSavedSearches).mockRejectedValue(new Error('offline'))
    const store = useSavedSearchStore()

    await expect(store.loadOnce()).resolves.toBe(false)
    await expect(store.loadOnce()).resolves.toBe(false)

    expect(fetchSavedSearches).toHaveBeenCalledOnce()
    expect(store.loaded).toBe(false)
  })

  it('uses each mutation response as the new ordered state', async () => {
    const renamed = { ...first, name: 'Family photos' }
    vi.mocked(createSavedSearch).mockResolvedValue([first])
    vi.mocked(renameSavedSearch).mockResolvedValue([renamed])
    vi.mocked(deleteSavedSearch).mockResolvedValue([])
    const store = useSavedSearchStore()

    await expect(
      store.create({
        name: first.name,
        context: first.context,
        query: first.query,
        sortOrder: first.sortOrder
      })
    ).resolves.toBe(true)
    expect(store.searches).toEqual([first])

    await expect(store.rename(first.id, renamed.name)).resolves.toBe(true)
    expect(store.searches).toEqual([renamed])

    await expect(store.remove(first.id)).resolves.toBe(true)
    expect(store.searches).toEqual([])
  })

  it('waits for the initial load before mutating so a stale GET cannot win', async () => {
    const pendingLoad = deferred<SavedSearch[]>()
    vi.mocked(fetchSavedSearches).mockReturnValue(pendingLoad.promise)
    vi.mocked(createSavedSearch).mockResolvedValue([first])
    const store = useSavedSearchStore()

    void store.loadOnce()
    const creation = store.create({
      name: first.name,
      context: first.context,
      query: first.query,
      sortOrder: first.sortOrder
    })
    expect(createSavedSearch).not.toHaveBeenCalled()

    pendingLoad.resolve([])
    await expect(creation).resolves.toBe(true)
    expect(createSavedSearch).toHaveBeenCalledOnce()
    expect(store.searches).toEqual([first])
  })

  it('optimistically reorders and restores the prior order when the API fails', async () => {
    vi.mocked(fetchSavedSearches).mockResolvedValue([first, second])
    vi.mocked(reorderSavedSearches).mockRejectedValue(new Error('write failed'))
    const store = useSavedSearchStore()
    await store.loadOnce()

    const promise = store.reorder([second.id, first.id])
    expect(store.searches.map((search) => search.id)).toEqual([second.id, first.id])

    await expect(promise).resolves.toBe(false)
    expect(store.searches.map((search) => search.id)).toEqual([first.id, second.id])
  })

  it('persists the order produced by a drag operation', async () => {
    vi.mocked(fetchSavedSearches).mockResolvedValue([first, second])
    vi.mocked(reorderSavedSearches).mockResolvedValue([second, first])
    const store = useSavedSearchStore()
    await store.loadOnce()

    await expect(store.reorder([second.id, first.id])).resolves.toBe(true)
    expect(reorderSavedSearches).toHaveBeenCalledWith([second.id, first.id])
    expect(store.searches).toEqual([second, first])
  })
})
