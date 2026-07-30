import type { GallerySortOrder } from '@/type/types'

export interface GallerySearchSubmission {
  query: string
  sortOrder: GallerySortOrder
}

export interface QuickSortPresentation {
  icon: string
  ariaLabel: string
}

export function parseGallerySortOrder(value: unknown): GallerySortOrder {
  return value === 'ascending' || value === 'random' ? value : 'descending'
}

export function nextQuickSortOrder(sortOrder: GallerySortOrder): GallerySortOrder {
  return sortOrder === 'descending' ? 'ascending' : 'descending'
}

export function getQuickSortPresentation(
  sortOrder: GallerySortOrder
): QuickSortPresentation {
  switch (sortOrder) {
    case 'ascending':
      return {
        icon: 'mdi-sort-ascending',
        ariaLabel: 'Currently sorted ascending. Switch to descending.'
      }
    case 'random':
      return {
        icon: 'mdi-shuffle',
        ariaLabel: 'Currently sorted randomly. Switch to descending.'
      }
    case 'descending':
      return {
        icon: 'mdi-sort-descending',
        ariaLabel: 'Currently sorted descending. Switch to ascending.'
    }
  }
}
