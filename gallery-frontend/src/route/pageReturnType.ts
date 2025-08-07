import { LocationQuery } from 'vue-router'

export interface PageReturnType {
  name: string
  params: {
    hash?: string | string[] | undefined
    subhash?: string | string[] | undefined
    albumId?: string | string[]
    shareId?: string | string[]
  }

  query: LocationQuery
}

declare module 'vue-router' {
  interface RouteMeta {
    isReadPage: boolean
    isViewPage: boolean
    baseName: BaseName
    getParentPage: (
      router: RouteLocationNormalizedLoadedGeneric,
      albumId?: string,
      shareId?: string
    ) => PageReturnType
    getChildPage:
      | ((router: RouteLocationNormalizedLoadedGeneric, hash: string | undefined) => PageReturnType)
      | undefined
  }
}

type BaseName =
  | 'gallery'
  | 'album'
  | 'tags'
  | 'login'
  | 'share'
  | 'links'
  | 'home'
  | 'all'
  | 'favorite'
  | 'archived'
  | 'trashed'
  | 'albums'
  | 'videos'
