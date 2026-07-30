import axios from 'axios'
import { savedSearchListSchema } from '@/type/schemas'
import type {
  GallerySortOrder,
  SavedSearch,
  SavedSearchContext
} from '@/type/types'

export interface CreateSavedSearchInput {
  name: string
  context: SavedSearchContext
  query: string
  sortOrder: GallerySortOrder
}

function parseSavedSearches(value: unknown): SavedSearch[] {
  return savedSearchListSchema.parse(value)
}

export async function fetchSavedSearches(): Promise<SavedSearch[]> {
  const response = await axios.get<unknown>('/get/saved_searches')
  return parseSavedSearches(response.data)
}

export async function createSavedSearch(input: CreateSavedSearchInput): Promise<SavedSearch[]> {
  const response = await axios.post<unknown>('/post/saved_searches', input)
  return parseSavedSearches(response.data)
}

export async function renameSavedSearch(id: string, name: string): Promise<SavedSearch[]> {
  const response = await axios.put<unknown>(`/put/saved_searches/${encodeURIComponent(id)}`, {
    name
  })
  return parseSavedSearches(response.data)
}

export async function reorderSavedSearches(ids: readonly string[]): Promise<SavedSearch[]> {
  const response = await axios.put<unknown>('/put/saved_searches/order', { ids })
  return parseSavedSearches(response.data)
}

export async function deleteSavedSearch(id: string): Promise<SavedSearch[]> {
  const response = await axios.delete<unknown>(
    `/delete/saved_searches/${encodeURIComponent(id)}`
  )
  return parseSavedSearches(response.data)
}
