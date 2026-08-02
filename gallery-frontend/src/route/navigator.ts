import { useCollectionReloadStore } from '@/store/collectionReloadStore'
import type { Router } from 'vue-router'

export async function navigateToAlbum(
  albumId: string,
  router: Router
): ReturnType<Router['push']> {
  const albumPath = `/albums/view/${albumId}/read`

  if (router.currentRoute.value.fullPath.startsWith('/albums')) {
    const collectionReloadStore = useCollectionReloadStore('mainId')
    collectionReloadStore.requestMainCollectionReload()
  }

  return router.push({ path: albumPath })
}
