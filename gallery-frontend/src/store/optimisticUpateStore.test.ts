import { beforeEach, describe, expect, it } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { BackendDataParser } from '@/type/schemas'
import type { EnrichedUnifiedData } from '@/type/types'
import { useDataStore } from './dataStore'
import { useOptimisticStore } from './optimisticUpateStore'

const image = (id: string): EnrichedUnifiedData => ({
  ...BackendDataParser.parse({
    type: 'image',
    id,
    width: 1,
    height: 1,
    ext: 'jpg',
    size: 1,
    pending: false,
    albums: [],
    alias: []
  }),
  thumbhashUrl: null,
  timestamp: 0
})

describe('loaded-only optimistic updates', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('touches only loaded items accepted by the selection matcher', () => {
    const data = useDataStore('tempId')
    const optimistic = useOptimisticStore('tempId')
    data.data.set(2, image('selected'))
    data.data.set(5, image('excluded'))

    optimistic.optimisticUpdateTags(
      {
        selection: { mode: 'allExcept', excludedIndices: [5] },
        addTagsArray: ['marker'],
        removeTagsArray: [],
        timestamp: 1
      },
      true
    )
    expect(data.data.get(2)?.tags).toContain('marker')
    expect(data.data.get(5)?.tags).not.toContain('marker')
    expect(optimistic.queueTagsUpdate).toHaveLength(0)

    optimistic.optimisticUpdateAlbums(
      {
        selection: { mode: 'explicit', indices: [5, 100_000] },
        addAlbumsArray: ['album'],
        removeAlbumsArray: [],
        timestamp: 1
      },
      true
    )
    const selected = data.data.get(2)
    const excluded = data.data.get(5)
    expect(selected?.type).toBe('image')
    expect(excluded?.type).toBe('image')
    if (selected?.type !== 'image' || excluded?.type !== 'image') {
      throw new Error('test fixture must contain images')
    }
    expect(selected.albums).not.toContain('album')
    expect(excluded.albums).toContain('album')
    expect(data.data.has(100_000)).toBe(false)
  })
})
