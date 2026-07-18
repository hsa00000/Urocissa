import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useCollectionStore } from './collectionStore'

describe('collection selection store', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('keeps Select All as a flag plus exceptions', () => {
    const store = useCollectionStore('tempId')
    store.selectAll()
    expect(store.selectedCount(1_000_000)).toBe(1_000_000)
    expect(store.selectionSet.size).toBe(0)
    store.deleteApi(12)
    store.deleteApi(25)
    expect(store.selectedCount(1_000_000)).toBe(999_998)
    expect(store.isSelected(12)).toBe(false)
    expect(store.descriptor()).toEqual({ mode: 'allExcept', excludedIndices: [12, 25] })
  })

  it('supports explicit selection, inverse, clear, and single selection', () => {
    const store = useCollectionStore('tempId')
    store.addApi(8)
    store.addApi(2)
    expect(store.selectedCount(10)).toBe(2)
    expect(store.descriptor()).toEqual({ mode: 'explicit', indices: [2, 8] })
    store.invertSelection()
    expect(store.selectedCount(10)).toBe(8)
    expect(store.isSelected(2)).toBe(false)
    store.clearSelection()
    store.addApi(6)
    expect(store.singleSelectedIndex(10)).toBe(6)
    store.clearSelection()
    expect(store.hasSelection(10)).toBe(false)
  })
})
