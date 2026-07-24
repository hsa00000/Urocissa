import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import axios from 'axios'
import { useSearchFacetStore } from './searchFacetStore'

describe('search facet store', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('loads every facet from the sole endpoint and sorts each list', async () => {
    const get = vi.spyOn(axios, 'get').mockResolvedValue({
      status: 200,
      data: {
        tags: [
          { value: 'zebra', count: 1 },
          { value: 'alpha', count: 2 }
        ],
        makes: [
          { value: 'Sony', count: 3 },
          { value: 'Canon', count: 4 }
        ],
        models: [
          { value: 'Z8', count: 1 },
          { value: 'A1', count: 2 }
        ]
      }
    })
    const store = useSearchFacetStore()

    await store.fetchFacets()

    expect(get).toHaveBeenCalledOnce()
    expect(get).toHaveBeenCalledWith('/get/get-search-facets')
    expect(store.tags.map((facet) => facet.value)).toEqual(['alpha', 'zebra'])
    expect(store.makes.map((facet) => facet.value)).toEqual(['Canon', 'Sony'])
    expect(store.models.map((facet) => facet.value)).toEqual(['A1', 'Z8'])
    expect(store.fetched).toBe(true)
  })

  it('loads scoped facets without replacing the shared aggregate state', async () => {
    const get = vi.spyOn(axios, 'get').mockResolvedValue({
      status: 200,
      data: {
        tags: [
          { value: 'zebra', count: 1 },
          { value: 'alpha', count: 2 }
        ],
        makes: [],
        models: []
      }
    })
    const store = useSearchFacetStore()

    const facets = await store.fetchFacetsForTrashState(true)

    expect(get).toHaveBeenCalledWith('/get/get-search-facets', { params: { trashed: true } })
    expect(facets?.tags.map((facet) => facet.value)).toEqual(['alpha', 'zebra'])
    expect(store.tags).toEqual([])
    expect(store.makes).toEqual([])
    expect(store.models).toEqual([])
    expect(store.fetched).toBe(false)
  })

  it('preserves camera metadata values that differ only by case', async () => {
    vi.spyOn(axios, 'get').mockResolvedValue({
      status: 200,
      data: {
        tags: [],
        makes: [
          { value: 'Canon', count: 3 },
          { value: 'CANON', count: 1 },
          { value: 'canon', count: 2 }
        ],
        models: [
          { value: 'R5', count: 4 },
          { value: 'r5', count: 1 }
        ]
      }
    })
    const store = useSearchFacetStore()

    await store.fetchFacets()

    expect(store.makes).toHaveLength(3)
    expect(store.makes).toEqual(
      expect.arrayContaining([
        { value: 'Canon', count: 3 },
        { value: 'CANON', count: 1 },
        { value: 'canon', count: 2 }
      ])
    )
    expect(store.models).toHaveLength(2)
    expect(store.models).toEqual(
      expect.arrayContaining([
        { value: 'R5', count: 4 },
        { value: 'r5', count: 1 }
      ])
    )
  })

  it('shares one in-flight request and skips later fetches after success', async () => {
    let resolveRequest: ((value: { status: number; data: unknown }) => void) | undefined
    const get = vi.spyOn(axios, 'get').mockImplementation(
      () =>
        new Promise((resolve) => {
          resolveRequest = resolve
        })
    )
    const store = useSearchFacetStore()
    const first = store.fetchFacets()
    const second = store.fetchFacets()

    expect(get).toHaveBeenCalledOnce()
    resolveRequest?.({ status: 200, data: { tags: [], makes: [], models: [] } })
    await Promise.all([first, second])
    await store.fetchFacets()

    expect(get).toHaveBeenCalledOnce()
  })

  it('applies authoritative tags without claiming all facets were fetched', () => {
    const store = useSearchFacetStore()
    store.applyTags([
      { value: 'second', count: 1 },
      { value: 'first', count: 2 }
    ])

    expect(store.tags.map((facet) => facet.value)).toEqual(['first', 'second'])
    expect(store.fetched).toBe(false)
  })

  it('can retry after an invalid response without exposing partial state', async () => {
    const get = vi
      .spyOn(axios, 'get')
      .mockResolvedValueOnce({
        status: 200,
        data: { tags: [], makes: [] }
      })
      .mockResolvedValueOnce({
        status: 200,
        data: { tags: [], makes: [], models: [] }
      })
    const store = useSearchFacetStore()

    await store.fetchFacets()
    expect(store.fetched).toBe(false)
    expect(store.tags).toEqual([])
    expect(store.makes).toEqual([])
    expect(store.models).toEqual([])

    await store.fetchFacets()
    expect(get).toHaveBeenCalledTimes(2)
    expect(store.fetched).toBe(true)
  })

  it('adds optimistic tags once and clears all state', async () => {
    vi.spyOn(axios, 'get').mockResolvedValue({
      status: 200,
      data: {
        tags: [{ value: 'existing', count: 2 }],
        makes: [{ value: 'Canon', count: 1 }],
        models: [{ value: 'R5', count: 1 }]
      }
    })
    const store = useSearchFacetStore()
    await store.fetchFacets()

    store.addOptimisticTag('new')
    store.addOptimisticTag('new')
    expect(store.tags).toEqual([
      { value: 'existing', count: 2 },
      { value: 'new', count: 1 }
    ])

    store.clearAll()
    expect(store.tags).toEqual([])
    expect(store.makes).toEqual([])
    expect(store.models).toEqual([])
    expect(store.fetched).toBe(false)
  })
})
