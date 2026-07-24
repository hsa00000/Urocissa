import { describe, expect, it, vi } from 'vitest'
import type { Router } from 'vue-router'
import {
  createFacetSearchLocation,
  searchByFacet,
  searchByTag,
  type FacetSearchField
} from './getter'

function route(
  baseName: 'share' | 'tags' | 'trashed',
  params: Record<string, string> = {}
): { meta: { baseName: 'share' | 'tags' | 'trashed' }; params: Record<string, string> } {
  return { meta: { baseName }, params }
}

describe('facet search routes', () => {
  it.each<[FacetSearchField, string]>([
    ['tag', 'family'],
    ['make', 'Canon'],
    ['model', 'EOS R5']
  ])('creates an All search location for %s values', (field, value) => {
    expect(createFacetSearchLocation(field, value, route('tags'))).toEqual({
      name: 'all',
      query: { search: `${field}:"${value}"` }
    })
  })

  it('escapes quotes and backslashes in facet values', () => {
    expect(createFacetSearchLocation('make', 'Canon "EOS" \\ camera', route('tags'))).toEqual({
      name: 'all',
      query: { search: 'make:"Canon \\"EOS\\" \\\\ camera"' }
    })
  })

  it('keeps tag searches inside the current share root', async () => {
    const push = vi.fn().mockResolvedValue(undefined)
    const router = {
      currentRoute: { value: route('share', { albumId: 'album-1', shareId: 'share-1' }) },
      push
    } as unknown as Router

    await searchByTag('family', router)

    expect(push).toHaveBeenCalledWith({
      name: 'share',
      params: { albumId: 'album-1', shareId: 'share-1' },
      query: { search: 'tag:"family"' }
    })
  })

  it('navigates make and model facets through the generic helper', async () => {
    const push = vi.fn().mockResolvedValue(undefined)
    const router = {
      currentRoute: { value: route('tags') },
      push
    } as unknown as Router

    await searchByFacet('model', 'R5', router)

    expect(push).toHaveBeenCalledWith({
      name: 'all',
      query: { search: 'model:"R5"' }
    })
  })

  it('uses the selected scope for facet searches', () => {
    expect(createFacetSearchLocation('tag', 'family', route('tags'), 'trashed')).toEqual({
      name: 'trashed',
      query: { search: 'tag:"family"' }
    })
  })

  it('keeps tag searches in the trashed scope from trashed detail pages', () => {
    expect(createFacetSearchLocation('tag', 'family', route('trashed'))).toEqual({
      name: 'trashed',
      query: { search: 'tag:"family"' }
    })
  })
})
