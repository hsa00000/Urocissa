import { describe, expect, it } from 'vitest'
import type { LocationQuery } from 'vue-router'
import { createGallerySearchRouteUpdate } from './gallerySearchRoute'

describe('gallery search route updates', () => {
  it('trims album searches while preserving the outer gallery and unrelated query state', () => {
    const currentQuery: LocationQuery = {
      search: 'tag:outer',
      locate: 'photo-id',
      priority_id: 'priority-id',
      subSearch: 'old'
    }

    expect(
      createGallerySearchRouteUpdate(currentQuery, 'subSearch', {
        query: '  tag:inside  ',
        sortOrder: 'descending'
      })
    ).toEqual({
      normalizedSearch: 'tag:inside',
      routeQuery: {
        search: 'tag:outer',
        locate: 'photo-id',
        priority_id: 'priority-id',
        subSearch: 'tag:inside'
      }
    })
    expect(currentQuery).toEqual({
      search: 'tag:outer',
      locate: 'photo-id',
      priority_id: 'priority-id',
      subSearch: 'old'
    })
  })

  it('clears only the selected search key', () => {
    expect(
      createGallerySearchRouteUpdate(
        {
          search: 'tag:outer',
          subSearch: 'tag:inside'
        },
        'subSearch',
        {
          query: '   ',
          sortOrder: 'descending'
        }
      )
    ).toEqual({
      normalizedSearch: '',
      routeQuery: {
        search: 'tag:outer'
      }
    })
  })

  it('updates main searches without disturbing an album search carried in the URL', () => {
    expect(
      createGallerySearchRouteUpdate(
        {
          search: 'old',
          subSearch: 'tag:inside'
        },
        'search',
        {
          query: 'new',
          sortOrder: 'descending'
        }
      )
    ).toEqual({
      normalizedSearch: 'new',
      routeQuery: {
        search: 'new',
        subSearch: 'tag:inside'
      }
    })
  })

  it.each([
    ['descending', undefined],
    ['ascending', 'ascending'],
    ['random', 'random']
  ] as const)('normalizes %s sorting and removes the legacy reverse parameter', (sortOrder, sort) => {
    const update = createGallerySearchRouteUpdate(
      {
        reverse: 'true',
        sort: 'random'
      },
      'search',
      {
        query: '',
        sortOrder
      }
    )

    expect(update.routeQuery).not.toHaveProperty('reverse')
    if (sort === undefined) {
      expect(update.routeQuery).not.toHaveProperty('sort')
    } else {
      expect(update.routeQuery.sort).toBe(sort)
    }
  })
})
