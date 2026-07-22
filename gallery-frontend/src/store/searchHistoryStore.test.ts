import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import {
  MAX_SEARCH_HISTORY_ENTRIES,
  SEARCH_HISTORY_STORAGE_KEY,
  useSearchHistoryStore
} from './searchHistoryStore'

function createMemoryStorage(): Storage {
  const data = new Map<string, string>()

  return {
    get length(): number {
      return data.size
    },
    clear(): void {
      data.clear()
    },
    getItem(key: string): string | null {
      return data.get(key) ?? null
    },
    key(index: number): string | null {
      return Array.from(data.keys())[index] ?? null
    },
    removeItem(key: string): void {
      data.delete(key)
    },
    setItem(key: string, value: string): void {
      data.set(key, value)
    }
  }
}

describe('search history store', () => {
  let storage: Storage

  beforeEach(() => {
    setActivePinia(createPinia())
    storage = createMemoryStorage()
    vi.stubGlobal('localStorage', storage)
  })

  afterEach(() => {
    vi.restoreAllMocks()
    vi.unstubAllGlobals()
  })

  it('loads normalized, unique history and ignores malformed entries', () => {
    storage.setItem(
      SEARCH_HISTORY_STORAGE_KEY,
      JSON.stringify([' first ', 12, 'second', 'first', '', null])
    )

    expect(useSearchHistoryStore().history).toEqual(['first', 'second'])
  })

  it('falls back to empty history for malformed storage data', () => {
    storage.setItem(SEARCH_HISTORY_STORAGE_KEY, '{not-json')
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)

    expect(useSearchHistoryStore().history).toEqual([])
    expect(warn).toHaveBeenCalledOnce()
  })

  it('deduplicates recent searches, moves them to the front, and caps the list', () => {
    const store = useSearchHistoryStore()
    for (let index = 0; index <= MAX_SEARCH_HISTORY_ENTRIES; index += 1) {
      store.add(`query-${index}`)
    }

    expect(store.history).toHaveLength(MAX_SEARCH_HISTORY_ENTRIES)
    expect(store.history[0]).toBe(`query-${MAX_SEARCH_HISTORY_ENTRIES}`)
    expect(store.history).not.toContain('query-0')

    store.add(' query-5 ')
    expect(store.history[0]).toBe('query-5')
    expect(store.history.filter((item) => item === 'query-5')).toHaveLength(1)
  })

  it('removes and clears entries while persisting each update', () => {
    const store = useSearchHistoryStore()
    store.add('first')
    store.add('second')
    store.remove(1)

    expect(store.history).toEqual(['second'])
    expect(JSON.parse(storage.getItem(SEARCH_HISTORY_STORAGE_KEY) ?? '[]')).toEqual(['second'])

    store.clear()
    expect(store.history).toEqual([])
    expect(storage.getItem(SEARCH_HISTORY_STORAGE_KEY)).toBe('[]')
  })

  it('continues working when localStorage is unavailable or rejects writes', () => {
    vi.stubGlobal('localStorage', undefined)
    const unavailableStore = useSearchHistoryStore()
    expect(() => {
      unavailableStore.add('offline')
    }).not.toThrow()
    expect(unavailableStore.history).toEqual(['offline'])

    setActivePinia(createPinia())
    const rejectingStorage = createMemoryStorage()
    vi.spyOn(rejectingStorage, 'setItem').mockImplementation(() => {
      throw new Error('Storage disabled')
    })
    vi.stubGlobal('localStorage', rejectingStorage)
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const rejectingStore = useSearchHistoryStore()

    expect(() => {
      rejectingStore.add('still-searches')
    }).not.toThrow()
    expect(rejectingStore.history).toEqual(['still-searches'])
    expect(warn).toHaveBeenCalledOnce()
  })
})
