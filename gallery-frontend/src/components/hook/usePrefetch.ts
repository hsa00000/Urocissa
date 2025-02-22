import { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import { watchDebounced } from '@vueuse/core'
import { Ref } from 'vue'
import { IsolationId, Prefetch } from '@/script/common/types'
import { prefetch } from '@/script/fetch/prefetch'
import axios from 'axios'
import { PublicConfigSchema } from '@/script/common/schemas'
import { useConfigStore } from '@/store/configStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useTagStore } from '@/store/tagStore'
import { useAlbumStore } from '@/store/albumStore'
import { useTokenStore } from '@/store/tokenStore'
import { fetchScrollbar } from '@/script/fetch/scrollbar'

export function usePrefetch(
  filterJsonString: string | null,
  windowWidth: Ref<number>,
  route: RouteLocationNormalizedLoadedGeneric,
  isolationId: IsolationId
) {
  const stopWatcher = watchDebounced(
    windowWidth,
    async () => {
      if (windowWidth.value > 0) {
        const priorityId = route.query.priorityId as string
        const reverse = route.query.reverse as string
        let locate: string | null = null

        if (route.meta.isViewPage) {
          locate = route.params.hash as string
        } else {
          const queryLocate = route.query.locate
          if (typeof queryLocate === 'string') {
            locate = queryLocate
          }
        }

        const prefetchReturn = await prefetch(filterJsonString, priorityId, reverse, locate)
        await handlePrefetchReturn(prefetchReturn, isolationId)
        stopWatcher() // Stop the watcher after prefetching
      }
    },
    { immediate: true, debounce: 75, maxWait: 1000 }
  )
}

// TODO optimize tags fetch
async function handlePrefetchReturn(result: Prefetch, isolationId: IsolationId) {
  const configStore = useConfigStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const albumStore = useAlbumStore('mainId')
  const initializedStore = useInitializedStore(isolationId)
  const tagStore = useTagStore('mainId')
  const tokenStore = useTokenStore(isolationId)
  try {
    const response = await axios.get('/get/get-config.json')
    const publicConfig = PublicConfigSchema.parse(response.data)
    configStore.disableImg = publicConfig.disableImg
  } catch (error) {
    console.error('Error fetching config:', error)
    throw error
  }

  prefetchStore.timestamp = result.timestamp
  prefetchStore.updateVisibleRowTrigger = !prefetchStore.updateVisibleRowTrigger
  prefetchStore.calculateLength(result.dataLength)
  prefetchStore.locateTo = result.locateTo
  tokenStore.timestampToken = result.token

  initializedStore.initialized = true

  // Perform initialization:
  if (!tagStore.fetched) {
    await tagStore.fetchTags()
  }
  if (!albumStore.fetched) {
    await albumStore.fetchAlbums()
  }

  await fetchScrollbar(isolationId)

  prefetchStore.updateFetchRowTrigger = !prefetchStore.updateFetchRowTrigger
}
