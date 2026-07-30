import { describe, expect, it } from 'vitest'
import {
  getQuickSortPresentation,
  nextQuickSortOrder,
  parseGallerySortOrder
} from './gallerySort'

describe('gallery sort helpers', () => {
  it('normalizes missing, invalid, and array route values to descending', () => {
    expect(parseGallerySortOrder(undefined)).toBe('descending')
    expect(parseGallerySortOrder('descending')).toBe('descending')
    expect(parseGallerySortOrder('ascending')).toBe('ascending')
    expect(parseGallerySortOrder('random')).toBe('random')
    expect(parseGallerySortOrder('newest')).toBe('descending')
    expect(parseGallerySortOrder(['ascending'])).toBe('descending')
  })

  it('toggles descending and ascending while random returns to descending', () => {
    expect(nextQuickSortOrder('descending')).toBe('ascending')
    expect(nextQuickSortOrder('ascending')).toBe('descending')
    expect(nextQuickSortOrder('random')).toBe('descending')
  })

  it('provides state-specific icons and accessible action labels', () => {
    expect(getQuickSortPresentation('descending')).toEqual({
      icon: 'mdi-sort-descending',
      ariaLabel: 'Currently sorted descending. Switch to ascending.'
    })
    expect(getQuickSortPresentation('ascending').icon).toBe('mdi-sort-ascending')
    expect(getQuickSortPresentation('random')).toEqual({
      icon: 'mdi-shuffle',
      ariaLabel: 'Currently sorted randomly. Switch to descending.'
    })
  })
})
