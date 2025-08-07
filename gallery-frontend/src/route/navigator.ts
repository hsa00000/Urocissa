import { useRerenderStore } from '@/store/rerenderStore'
import { Router } from 'vue-router'

export async function navigateToAlbum(albumId: string, router: Router) {
  const albumPath = `/gallery/view/${albumId}/read?type=albums` // Adjust the path as necessary
  if (router.currentRoute.value.fullPath.startsWith('/gallery') && router.currentRoute.value.query.type === 'albums') {
    const rerenderStore = useRerenderStore('mainId')
    rerenderStore.rerenderHome()
  }
  await router.push({ path: albumPath })
}

export async function leave(router: Router) {
  const route = router.currentRoute.value
  let parentPage
  if (typeof route.params.albumId == 'string' && typeof route.params.shareId == 'string') {
    parentPage = route.meta.getParentPage(route, route.params.albumId, route.params.shareId)
  } else {
    parentPage = route.meta.getParentPage(route)
  }

  await router.push(parentPage)
}
