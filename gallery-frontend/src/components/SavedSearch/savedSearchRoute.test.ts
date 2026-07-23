import { describe, expect, it } from 'vitest'
import type { LocationQuery } from 'vue-router'
import type { SavedSearch } from '@/type/types'
import {
  SAVED_SEARCH_ADDED_MESSAGE,
  canSaveSearch,
  createSavedSearchLocation,
  getSavedSearchContext,
  isSavedSearchActive
} from './savedSearchRoute'

const search: SavedSearch = {
  id: '020b6f4f-5c28-4f8c-81f8-bc22949f1ee8',
  name: 'Family',
  context: 'favorite',
  query: 'tag:family'
}

function route(baseName: string, query: LocationQuery = {}) {
  return { meta: { baseName }, query }
}

describe('saved search routes', () => {
  it('accepts gallery contexts and rejects utility pages', () => {
    expect(getSavedSearchContext(route('favorite'))).toBe('favorite')
    expect(getSavedSearchContext(route('links'))).toBeNull()
    expect(canSaveSearch('home', ' tag:family ')).toBe(true)
    expect(canSaveSearch('home', '   ')).toBe(false)
    expect(canSaveSearch(null, 'tag:family')).toBe(false)
  })

  it('creates a clean target without transient query parameters', () => {
    expect(createSavedSearchLocation(search)).toEqual({
      name: 'favorite',
      query: { search: 'tag:family' }
    })
  })

  it('matches context and query exactly', () => {
    expect(isSavedSearchActive(search, route('favorite', { search: 'tag:family' }))).toBe(true)
    expect(
      isSavedSearchActive(
        search,
        route('favorite', { search: 'tag:family', locate: 'temporary-photo' })
      )
    ).toBe(true)
    expect(isSavedSearchActive(search, route('home', { search: 'tag:family' }))).toBe(false)
    expect(isSavedSearchActive(search, route('favorite', { search: 'other' }))).toBe(false)
  })

  it('keeps the required success snackbar copy stable', () => {
    expect(SAVED_SEARCH_ADDED_MESSAGE).toBe('Saved search added to drawer.')
  })
})
