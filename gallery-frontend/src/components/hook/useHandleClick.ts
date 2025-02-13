import { Router, RouteLocationNormalizedLoaded } from 'vue-router'
import { useCollectionStore } from '@/store/collectionStore'
import { useDataStore } from '@/store/dataStore'
import { IsolationId } from '@/script/common/types'

export function useHandleClick(
  router: Router,
  route: RouteLocationNormalizedLoaded,
  isolationId: IsolationId
) {
  const handleClick = (event: MouseEvent, currentIndex: number) => {
    const collectionStore = useCollectionStore(isolationId)
    if (collectionStore.editModeOn) {
      if (event.shiftKey && collectionStore.lastClick !== null) {
        const start = Math.min(collectionStore.lastClick, currentIndex)
        const end = Math.max(collectionStore.lastClick, currentIndex)
        let allInCollection = true

        for (let i = start; i <= end; i++) {
          if (!collectionStore.editModeCollection.has(i)) {
            allInCollection = false
            break
          }
        }
        if (allInCollection) {
          for (let i = start; i <= end; i++) {
            collectionStore.deleteApi(i)
          }
          collectionStore.lastClick = null
        } else {
          for (let i = start; i <= end; i++) {
            collectionStore.addApi(i)
          }
          collectionStore.lastClick = currentIndex
        }
      } else {
        if (collectionStore.editModeCollection.has(currentIndex)) {
          collectionStore.deleteApi(currentIndex)
          collectionStore.lastClick = null
        } else {
          collectionStore.addApi(currentIndex)
          collectionStore.lastClick = currentIndex
        }
      }
    } else {
      // collectionStore.editModeOn === false
      const dataStore = useDataStore(isolationId)
      const abstractData = dataStore.data.get(currentIndex)
      if (abstractData) {
        const hashOrId = abstractData.database
          ? abstractData.database.hash
          : abstractData.album
          ? abstractData.album.id
          : undefined
        if (hashOrId !== undefined) {
          const page = route.meta.getChildPage(route, hashOrId)

          router
            .push(page)
            .then(() => ({}))
            .catch((error: unknown) => {
              console.error('Navigation Error:', error)
            })
        } else {
          console.error('Abstract Data Details:', abstractData)
          throw new Error(
            'Navigation failed: Neither "abstractData.database.hash" nor "abstractData.album.id" is defined.'
          )
        }
      } else {
        console.warn(`abstractData with index ${currentIndex} is not fetched.`)
      }
    }
    if (collectionStore.editModeCollection.size === 0) {
      collectionStore.editModeOn = false
    }
  }

  return { handleClick }
}
