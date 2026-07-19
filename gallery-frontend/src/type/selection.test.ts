import { describe, expect, it } from 'vitest'
import {
  createSelectionMatcher,
  normalizeSelection,
  selectionBatches,
  selectionCount,
  selectionIncludes
} from './selection'

describe('selection descriptors', () => {
  it('normalizes legacy arrays and preserves the new wire descriptor', () => {
    expect(normalizeSelection([3, 1])).toEqual({ mode: 'explicit', indices: [3, 1] })
    const descriptor = { mode: 'allExcept' as const, excludedIndices: [2, 8] }
    expect(normalizeSelection(descriptor)).toBe(descriptor)
  })

  it('matches explicit and all-with-exceptions selections', () => {
    expect(selectionIncludes({ mode: 'explicit', indices: [1, 4] }, 4)).toBe(true)
    expect(selectionIncludes({ mode: 'explicit', indices: [1, 4] }, 3)).toBe(false)
    const matches = createSelectionMatcher({ mode: 'allExcept', excludedIndices: [1, 4] })
    expect(matches(0)).toBe(true)
    expect(matches(4)).toBe(false)
  })

  it('counts a frozen selection without duplicate or out-of-range entries', () => {
    expect(selectionCount({ mode: 'explicit', indices: [1, 1, 4, 99] }, 6)).toBe(2)
    expect(selectionCount({ mode: 'allExcept', excludedIndices: [1, 1, 4, 99] }, 6)).toBe(4)
  })

  it('expands lazily in fixed-size batches without duplicates or invalid indices', () => {
    expect([
      ...selectionBatches({ mode: 'explicit', indices: [5, 1, 1, 99, -1, 3] }, 8, 2)
    ]).toEqual([[1, 3], [5]])
    expect([
      ...selectionBatches({ mode: 'allExcept', excludedIndices: [1, 4] }, 7, 3)
    ]).toEqual([[0, 2, 3], [5, 6]])
    expect(() => [
      ...selectionBatches({ mode: 'explicit', indices: [] }, 10, 0)
    ]).toThrow(RangeError)
  })
})
