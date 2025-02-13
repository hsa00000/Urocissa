import { useRerenderStore } from '@/store/rerenderStore'
import { RouteLocationNormalizedLoadedGeneric, Router } from 'vue-router'

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

export async function navigateToAlbum(albumId: string, router: Router) {
  const albumPath = `/albums/view/${albumId}/read` // Adjust the path as necessary
  if (router.currentRoute.value.fullPath.startsWith('/albums')) {
    const rerenderStore = useRerenderStore('mainId')
    rerenderStore.rerenderHome()
  }
  await router.push({ path: albumPath })
}

export async function leave(router: Router) {
  const route = router.currentRoute.value
  const parentPage = route.meta.getParentPage(route)
  await router.push(parentPage)
}
