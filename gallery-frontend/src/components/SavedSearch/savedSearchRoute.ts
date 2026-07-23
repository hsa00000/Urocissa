import type { RouteLocationRaw } from 'vue-router'
import type { SavedSearch, SavedSearchContext } from '@/type/types'

const contextSet = new Set<SavedSearchContext>([
  'home',
  'all',
  'favorite',
  'archived',
  'trashed',
  'albums',
  'videos'
])

interface SavedSearchRouteLike {
  meta: {
    baseName?: unknown
  }
  query: {
    search?: unknown
  }
}

export const SAVED_SEARCH_ADDED_MESSAGE = 'Saved search added to drawer.'

export function getSavedSearchContext(route: SavedSearchRouteLike): SavedSearchContext | null {
  const baseName = route.meta.baseName
  return typeof baseName === 'string' && contextSet.has(baseName as SavedSearchContext)
    ? (baseName as SavedSearchContext)
    : null
}

export function canSaveSearch(
  context: SavedSearchContext | null,
  query: string | null
): boolean {
  return context !== null && query !== null && query.trim() !== ''
}

export function createSavedSearchLocation(search: SavedSearch): RouteLocationRaw {
  return {
    name: search.context,
    query: { search: search.query }
  }
}

export function isSavedSearchActive(search: SavedSearch, route: SavedSearchRouteLike): boolean {
  return getSavedSearchContext(route) === search.context && route.query.search === search.query
}
