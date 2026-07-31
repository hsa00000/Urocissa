import { describe, expect, it } from 'vitest'
import { calculateThumbnailSize } from './thumbnailResize'

describe('calculateThumbnailSize', () => {
  it('fits landscape and portrait images inside the requested display box', () => {
    expect(calculateThumbnailSize(4000, 3000, 400, 300)).toEqual({
      width: 400,
      height: 300
    })
    expect(calculateThumbnailSize(3000, 4000, 400, 300)).toEqual({
      width: 225,
      height: 300
    })
  })

  it('does not upscale an image that already fits', () => {
    expect(calculateThumbnailSize(100, 50, 400, 300)).toEqual({
      width: 100,
      height: 50
    })
  })

  it('preserves cover-mode aspect ratios after its bounds are calculated', () => {
    expect(calculateThumbnailSize(4000, 3000, 1600 / 3, 400)).toEqual({
      width: 533,
      height: 399
    })
  })

  it('rejects invalid source or target dimensions', () => {
    expect(() => calculateThumbnailSize(0, 100, 50, 50)).toThrow(
      'thumbnail dimensions must be positive'
    )
    expect(() => calculateThumbnailSize(100, 100, -1, 50)).toThrow(
      'thumbnail dimensions must be positive'
    )
  })
})
