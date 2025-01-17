import { useRerenderStore } from '@/store/rerenderStore'
import { RouteLocationNormalizedLoadedGeneric, Router } from 'vue-router'

export function leaveRead(route: RouteLocationNormalizedLoadedGeneric) {
  return {
    name: `${route.meta.baseName}ViewPage`,
    params: {
      hash: route.params.hash
    },
    query: route.query
  }
}

export function intoViewPage(route: RouteLocationNormalizedLoadedGeneric, hashOrSubhash: string) {
  if (!route.meta.isReadPage) {
    return {
      name: `${route.meta.baseName}ViewPage`,
      params: { hash: hashOrSubhash },
      query: route.query
    }
  } else {
    return {
      name: `${route.meta.baseName}ReadViewPage`,
      params: { hash: route.meta.hash as string, subhash: hashOrSubhash },
      query: route.query
    }
  }
}

export function leaveViewPage(route: RouteLocationNormalizedLoadedGeneric) {
  if (!route.meta.isReadPage) {
    return { name: route.meta.baseName, query: route.query }
  } else {
    return {
      name: `${route.meta.baseName}ReadPage`,
      params: {
        hash: route.params.hash
      },
      query: route.query
    }
  }
}

export async function navigateToAlbum(albumId: string, router: Router) {
  const albumPath = `/albums/view/${albumId}/read` // Adjust the path as necessary
  if (router.currentRoute.value.fullPath.startsWith('/albums')) {
    const rerenderStore = useRerenderStore('mainId')
    rerenderStore.rerenderHome()
  }
  await router.push({ path: albumPath })
}
