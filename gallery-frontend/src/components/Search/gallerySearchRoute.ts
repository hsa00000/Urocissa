import type { LocationQuery, LocationQueryRaw } from 'vue-router'
import type { GallerySearchSubmission } from '@/script/utils/gallerySort'

export type GallerySearchQueryKey = 'search' | 'subSearch'

export interface GallerySearchRouteUpdate {
  normalizedSearch: string
  routeQuery: LocationQueryRaw
}

export function createGallerySearchRouteUpdate(
  currentQuery: LocationQuery,
  searchKey: GallerySearchQueryKey,
  submission: GallerySearchSubmission
): GallerySearchRouteUpdate {
  const normalizedSearch = submission.query.trim()
  const routeQuery: LocationQueryRaw = { ...currentQuery }

  if (normalizedSearch === '') {
    if (searchKey === 'search') {
      delete routeQuery.search
    } else {
      delete routeQuery.subSearch
    }
  } else {
    routeQuery[searchKey] = normalizedSearch
  }

  delete routeQuery.reverse
  if (submission.sortOrder === 'descending') {
    delete routeQuery.sort
  } else {
    routeQuery.sort = submission.sortOrder
  }

  return {
    normalizedSearch,
    routeQuery
  }
}
