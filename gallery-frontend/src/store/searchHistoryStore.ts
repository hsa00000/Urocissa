import { defineStore } from 'pinia'
import { shallowRef } from 'vue'

export const SEARCH_HISTORY_STORAGE_KEY = 'urocissa_search_history'
export const MAX_SEARCH_HISTORY_ENTRIES = 15

function normalizeStoredHistory(value: unknown): string[] {
  if (!Array.isArray(value)) return []

  const history: string[] = []
  for (const item of value) {
    if (typeof item !== 'string') continue

    const query = item.trim()
    if (query === '' || history.includes(query)) continue

    history.push(query)
    if (history.length === MAX_SEARCH_HISTORY_ENTRIES) break
  }
  return history
}

function loadHistory(): string[] {
  try {
    if (typeof localStorage === 'undefined') return []

    const raw = localStorage.getItem(SEARCH_HISTORY_STORAGE_KEY)
    if (raw === null) return []
    return normalizeStoredHistory(JSON.parse(raw) as unknown)
  } catch (error) {
    console.warn('[searchHistoryStore] Failed to load search history', error)
    return []
  }
}

function saveHistory(history: readonly string[]): void {
  try {
    if (typeof localStorage === 'undefined') return
    localStorage.setItem(SEARCH_HISTORY_STORAGE_KEY, JSON.stringify(history))
  } catch (error) {
    console.warn('[searchHistoryStore] Failed to save search history', error)
  }
}

export const useSearchHistoryStore = defineStore('searchHistory', () => {
  const history = shallowRef<string[]>(loadHistory())

  function add(query: string): void {
    const normalizedQuery = query.trim()
    if (normalizedQuery === '') return

    history.value = [
      normalizedQuery,
      ...history.value.filter((item) => item !== normalizedQuery)
    ].slice(0, MAX_SEARCH_HISTORY_ENTRIES)
    saveHistory(history.value)
  }

  function remove(index: number): void {
    if (index < 0 || index >= history.value.length) return
    history.value = history.value.filter((_, itemIndex) => itemIndex !== index)
    saveHistory(history.value)
  }

  function clear(): void {
    history.value = []
    saveHistory(history.value)
  }

  return {
    history,
    add,
    remove,
    clear
  }
})
