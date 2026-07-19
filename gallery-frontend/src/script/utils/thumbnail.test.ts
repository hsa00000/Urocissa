import { describe, expect, it } from 'vitest'
import { getThumbnailSrc } from './thumbnail'

const hash = '0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef'

describe('getThumbnailSrc', () => {
  it('keeps the legacy filename for version zero', () => {
    expect(getThumbnailSrc(hash, 0)).toBe(`/object/compressed/01/${hash}.jpg`)
  })

  it('uses an immutable versioned filename after replacement', () => {
    expect(getThumbnailSrc(hash, 42)).toBe(`/object/compressed/01/${hash}-v42.jpg`)
  })
})
