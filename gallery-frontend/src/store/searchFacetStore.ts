import type { FacetValueInfo, SearchFacets } from '@type/types'
import { searchFacetsSchema } from '@type/schemas'
import axios from 'axios'
import { defineStore } from 'pinia'
import { shallowRef } from 'vue'
import { tryWithMessageStore } from '@/script/utils/try_catch'

function sortFacetValues(values: readonly FacetValueInfo[]): FacetValueInfo[] {
  return [...values].sort((left, right) => left.value.localeCompare(right.value))
}

function sortFacets(facets: SearchFacets): SearchFacets {
  return {
    tags: sortFacetValues(facets.tags),
    makes: sortFacetValues(facets.makes),
    models: sortFacetValues(facets.models)
  }
}

export const useSearchFacetStore = defineStore('searchFacets', () => {
  const tags = shallowRef<FacetValueInfo[]>([])
  const makes = shallowRef<FacetValueInfo[]>([])
  const models = shallowRef<FacetValueInfo[]>([])
  const fetched = shallowRef(false)
  let pendingFetch: Promise<void> | null = null

  async function requestFacets(trashed?: boolean): Promise<SearchFacets> {
    const response =
      trashed === undefined
        ? await axios.get('/get/get-search-facets')
        : await axios.get('/get/get-search-facets', { params: { trashed } })
    if (response.status !== 200) {
      throw new Error('Network response was not ok')
    }

    return sortFacets(searchFacetsSchema.parse(response.data))
  }

  async function fetchFacets(): Promise<void> {
    if (fetched.value) return
    if (pendingFetch !== null) return pendingFetch

    pendingFetch = (async () => {
      await tryWithMessageStore('mainId', async () => {
        const facets = await requestFacets()
        tags.value = sortFacetValues(facets.tags)
        makes.value = sortFacetValues(facets.makes)
        models.value = sortFacetValues(facets.models)
        fetched.value = true
      })
    })()

    try {
      await pendingFetch
    } finally {
      pendingFetch = null
    }
  }

  async function fetchFacetsForTrashState(trashed: boolean): Promise<SearchFacets | undefined> {
    return tryWithMessageStore('mainId', () => requestFacets(trashed))
  }

  function applyTags(nextTags: readonly FacetValueInfo[]): void {
    tags.value = sortFacetValues(nextTags)
  }

  function addOptimisticTag(value: string): void {
    if (tags.value.some((tag) => tag.value === value)) return
    tags.value = sortFacetValues([...tags.value, { value, count: 1 }])
  }

  function clearAll(): void {
    tags.value = []
    makes.value = []
    models.value = []
    fetched.value = false
  }

  return {
    tags,
    makes,
    models,
    fetched,
    fetchFacets,
    fetchFacetsForTrashState,
    applyTags,
    addOptimisticTag,
    clearAll
  }
})
