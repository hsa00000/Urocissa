import { describe, expect, it } from 'vitest'
import { createRootRouteKey } from './rootRouteKey'

const common = {
  baseName: 'home',
  concurrencyNumber: 4
}

describe('root route key', () => {
  it('changes only for a different root page or worker topology', () => {
    const current = createRootRouteKey(common)

    expect(createRootRouteKey(common)).toBe(current)
    expect(createRootRouteKey({ ...common, baseName: 'archived' })).not.toBe(current)
    expect(createRootRouteKey({ ...common, concurrencyNumber: 8 })).not.toBe(current)
  })
})
