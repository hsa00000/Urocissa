import { useRerenderStore } from '@/store/rerenderStore'
import type { Router } from 'vue-router'
import type { NavigationFailure } from 'vue-router'

export async function navigateToAlbum(
  albumId: string,
  router: Router
): Promise<void | NavigationFailure> {
  const albumPath = `/albums/view/${albumId}/read`

  if (router.currentRoute.value.fullPath.startsWith('/albums')) {
    const rerenderStore = useRerenderStore('mainId')
    rerenderStore.rerenderHome()
  }

  return router.push({ path: albumPath })
}
