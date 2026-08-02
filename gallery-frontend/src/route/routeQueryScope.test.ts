import { describe, expect, it } from 'vitest'
import { withoutReaderOnlyQuery } from './routeQueryScope'

describe('nested route query scope', () => {
  it('keeps reader controls off Level 2 and Level 1 history entries', () => {
    expect(
      withoutReaderOnlyQuery({
        search: 'main search',
        subSearch: 'reader search',
        sort: 'random',
        locate: 'child-id',
        priority_id: 'priority-id',
        unrelated: 'kept'
      })
    ).toEqual({
      search: 'main search',
      unrelated: 'kept'
    })
  })
})

