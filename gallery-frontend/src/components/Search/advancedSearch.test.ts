import { describe, expect, it } from 'vitest'
import {
  buildAdvancedSearchFilter,
  createEmptyAdvancedSearchCriteria,
  type AdvancedSearchCriteria
} from './advancedSearch'

function criteria(overrides: Partial<AdvancedSearchCriteria> = {}): AdvancedSearchCriteria {
  return { ...createEmptyAdvancedSearchCriteria(), ...overrides }
}

describe('advanced search filter builder', () => {
  it('returns an empty filter when every criterion is empty or all', () => {
    expect(buildAdvancedSearchFilter(criteria())).toBe('')
  })

  it('returns a single atomic expression without an and wrapper', () => {
    expect(buildAdvancedSearchFilter(criteria({ filename: '  vacation/photo.jpg  ' }))).toBe(
      'path:"vacation/photo.jpg"'
    )
  })

  it('trims and escapes quotes and backslashes', () => {
    expect(buildAdvancedSearchFilter(criteria({ keyword: '  C:\\photos "summer"  ' }))).toBe(
      'any:"C:\\\\photos \\"summer\\""'
    )
  })

  it('combines populated fields and includes the album media type', () => {
    expect(
      buildAdvancedSearchFilter(
        criteria({
          keyword: 'sunset',
          tag: 'family',
          extension: 'jpg',
          cameraMake: 'Canon',
          cameraModel: 'R5',
          mediaType: 'album'
        })
      )
    ).toBe(
      'and(any:"sunset", tag:"family", ext:"jpg", make:"Canon", model:"R5", type:"album")'
    )
  })
})
