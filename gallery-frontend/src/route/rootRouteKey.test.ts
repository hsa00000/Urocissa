import { describe, expect, it } from 'vitest'
import { createRootRouteKey } from './rootRouteKey'

const common = {
  baseName: 'home',
  concurrencyNumber: 4,
  homeKey: false
}

describe('root route key', () => {
  it('stays stable when a reader-only query changes', () => {
    const before = createRootRouteKey({ ...common, level: 3, query: {} })
    const after = createRootRouteKey({
      ...common,
      level: 3,
      query: { subSearch: 'tag:family', sort: 'random' }
    })

    expect(after).toBe(before)
  })

  it('still reloads Level 1 for its own collection query', () => {
    const before = createRootRouteKey({ ...common, level: 1, query: {} })
    const after = createRootRouteKey({ ...common, level: 1, query: { sort: 'random' } })

    expect(after).not.toBe(before)
  })

  it('keeps the Level 1 search identity while a reader is open', () => {
    const before = createRootRouteKey({
      ...common,
      level: 3,
      query: { search: 'main search' }
    })
    const after = createRootRouteKey({
      ...common,
      level: 3,
      query: { search: 'main search', sort: 'random' }
    })

    expect(after).toBe(before)
  })

  it('does not collide when query values contain delimiters or arrays', () => {
    const delimiterLeft = createRootRouteKey({
      ...common,
      level: 1,
      query: { search: 'a-b', locate: 'c' }
    })
    const delimiterRight = createRootRouteKey({
      ...common,
      level: 1,
      query: { search: 'a', locate: 'b-c' }
    })
    const oneArrayValue = createRootRouteKey({
      ...common,
      level: 1,
      query: { search: ['a,b'] }
    })
    const twoArrayValues = createRootRouteKey({
      ...common,
      level: 1,
      query: { search: ['a', 'b'] }
    })

    expect(delimiterLeft).not.toBe(delimiterRight)
    expect(oneArrayValue).not.toBe(twoArrayValues)
  })
})
