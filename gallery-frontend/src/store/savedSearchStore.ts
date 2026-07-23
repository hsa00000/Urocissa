import { readonly, shallowRef } from 'vue'
import { defineStore } from 'pinia'
import {
  createSavedSearch,
  deleteSavedSearch,
  fetchSavedSearches,
  renameSavedSearch,
  reorderSavedSearches,
  type CreateSavedSearchInput
} from '@/api/savedSearches'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SavedSearch } from '@/type/types'

export const useSavedSearchStore = defineStore('savedSearch', () => {
  const searches = shallowRef<SavedSearch[]>([])
  const loadAttempted = shallowRef(false)
  const loaded = shallowRef(false)
  const loading = shallowRef(false)
  const mutating = shallowRef(false)
  let loadPromise: Promise<boolean> | null = null

  function loadOnce(): Promise<boolean> {
    if (loadPromise !== null) return loadPromise

    loadAttempted.value = true
    loadPromise = performLoad()
    return loadPromise
  }

  async function performLoad(): Promise<boolean> {
    loading.value = true
    try {
      const result = await tryWithMessageStore('mainId', fetchSavedSearches)
      if (result === undefined) return false
      searches.value = result
      loaded.value = true
      return true
    } finally {
      loading.value = false
    }
  }

  async function create(input: CreateSavedSearchInput): Promise<boolean> {
    return runMutation(() => createSavedSearch(input))
  }

  async function rename(id: string, name: string): Promise<boolean> {
    return runMutation(() => renameSavedSearch(id, name))
  }

  async function remove(id: string): Promise<boolean> {
    return runMutation(() => deleteSavedSearch(id))
  }

  async function reorder(ids: readonly string[]): Promise<boolean> {
    if (mutating.value) return false

    mutating.value = true
    try {
      if (loading.value && loadPromise !== null) await loadPromise

      const previous = searches.value
      const byId = new Map(previous.map((search) => [search.id, search]))
      const next = ids.map((id) => byId.get(id)).filter((search) => search !== undefined)
      if (next.length !== previous.length || new Set(ids).size !== previous.length) return false
      if (next.every((search, index) => search === previous[index])) return true

      searches.value = next
      const result = await tryWithMessageStore('mainId', () => reorderSavedSearches(ids))
      if (result === undefined) {
        searches.value = previous
        return false
      }
      searches.value = result
      loaded.value = true
      return true
    } finally {
      mutating.value = false
    }
  }

  async function runMutation(request: () => Promise<SavedSearch[]>): Promise<boolean> {
    if (mutating.value) return false

    mutating.value = true
    try {
      if (loading.value && loadPromise !== null) await loadPromise

      const result = await tryWithMessageStore('mainId', request)
      if (result === undefined) return false
      searches.value = result
      loaded.value = true
      return true
    } finally {
      mutating.value = false
    }
  }

  return {
    searches: readonly(searches),
    loadAttempted: readonly(loadAttempted),
    loaded: readonly(loaded),
    loading: readonly(loading),
    mutating: readonly(mutating),
    loadOnce,
    create,
    rename,
    remove,
    reorder
  }
})
