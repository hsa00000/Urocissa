import { computed, onScopeDispose, shallowRef, watch } from 'vue'
import type { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import { getShareInfo } from '@/db/db'
import { useShareStore } from '@/store/shareStore'
import { escapeAndWrap } from '@/script/utils/escape'

export function useShareRouteContext(route: RouteLocationNormalizedLoadedGeneric) {
  const shareStore = useShareStore('mainId')
  const basicString = shallowRef<string>()
  const contextReady = shallowRef(false)
  let generation = 0

  const contextKey = computed(() =>
    JSON.stringify([
      shareStore.albumId,
      shareStore.shareId,
      shareStore.password
    ])
  )

  watch(
    [() => route.params.albumId, () => route.params.shareId],
    async ([albumIdParam, shareIdParam], _previous, onCleanup) => {
      generation += 1
      const requestGeneration = generation
      onCleanup(() => {
        if (requestGeneration === generation) generation += 1
      })

      contextReady.value = false
      basicString.value = undefined
      shareStore.clearActiveContext()

      if (typeof albumIdParam !== 'string' || typeof shareIdParam !== 'string') {
        console.error(
          `(albumId, shareId) is (${String(albumIdParam)}, ${String(shareIdParam)})`
        )
        return
      }

      shareStore.albumId = albumIdParam
      shareStore.shareId = shareIdParam

      try {
        const savedInfo = await getShareInfo(albumIdParam, shareIdParam)
        if (requestGeneration !== generation) return
        shareStore.password = savedInfo?.password ?? null
      } catch (error) {
        if (requestGeneration !== generation) return
        console.error('Failed to restore share credentials:', error)
        shareStore.password = null
      }

      try {
        await shareStore.syncShareInfoToIndexedDB()
      } catch (error) {
        if (requestGeneration !== generation) return
        console.error('Failed to persist share credentials:', error)
      }

      if (requestGeneration !== generation) return
      contextReady.value = true
      basicString.value = `and(trashed:false, album:${escapeAndWrap(albumIdParam)})`
    },
    { immediate: true }
  )

  watch(
    () => shareStore.password,
    async () => {
      if (!contextReady.value) return
      try {
        await shareStore.syncShareInfoToIndexedDB()
      } catch (error) {
        console.error('Failed to persist share credentials:', error)
      }
    }
  )

  onScopeDispose(() => {
    generation += 1
    contextReady.value = false
    basicString.value = undefined
    // Keep IndexedDB credentials for other tabs, but never let share headers
    // leak into authenticated routes after this share page is left.
    shareStore.clearActiveContext()
  })

  return { basicString, contextKey }
}
