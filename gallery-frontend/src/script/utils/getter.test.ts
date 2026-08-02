import { describe, expect, it } from 'vitest'
import { getRouteResourceId } from './getter'

describe('nested route resource identity', () => {
  it('returns the deepest matched resource ID', () => {
    expect(
      getRouteResourceId({
        meta: { level: 4 },
        params: { hash: 'album-id', subhash: 'media-id' }
      })
    ).toBe('media-id')
    expect(
      getRouteResourceId({ meta: { level: 2 }, params: { hash: 'parent-id' } })
    ).toBe('parent-id')
  })

  it('does not let a Level 4 action fall back to its parent album', () => {
    expect(
      getRouteResourceId({ meta: { level: 4 }, params: { hash: 'album-id' } })
    ).toBeUndefined()
  })
})
