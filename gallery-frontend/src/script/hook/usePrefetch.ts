
import { watchDebounced } from '@vueuse/core'
import { Ref } from 'vue'
import { IsolationId, PrefetchReturn } from '@type/types'
import { prefetch } from '@/api/fetchPrefetch'
import { useConfigStore } from '@/store/configStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { useAlbumStore } from '@/store/albumStore'
import { fetchScrollbar } from '@/api/fetchScrollbar'
import { useShareStore } from '@/store/shareStore'
import { useTokenStore } from '@/store/tokenStore'
import { RouteLocationNormalizedLoadedGeneric } from 'vue-router'
import type { GallerySortOrder } from '@type/types'
import { parseGallerySortOrder } from '@/script/utils/gallerySort'

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
        // Note: Check if 'priorityId' vs 'priority_id' matches your route definition in App.vue
        const priorityId =
          typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
        const sortOrder = parseGallerySortOrder(route.query.sort)
        let locate: string | null = null

        // add locate to query string if user enter view page directly
        if (
          isolationId === 'subId' &&
          route.meta.level === 4 &&
          typeof route.params.subhash === 'string'
        ) {
          locate = route.params.subhash
        } else if (isolationId === 'mainId' && typeof route.params.hash === 'string') {
          locate = route.params.hash
        } else if (typeof route.query.locate === 'string') {
          locate = route.query.locate
        }

        // Parallel Execution: Run Config chain and Prefetch chain simultaneously
        await Promise.all([
          processConfigChain(isolationId),
          processPrefetchChain(
            filterJsonString,
            priorityId,
            sortOrder,
            locate,
            isolationId,
            route
          )
        ])

        stopWatcher() // Stop the watcher after everything is done
      }
    },
    { immediate: true, debounce: 75, maxWait: 1000 }
  )
}

/**
 * Chain 1: Handles Configuration fetching.
 * Independent of prefetch data.
 */
async function processConfigChain(isolationId: IsolationId) {
  const configStore = useConfigStore(isolationId)
  await configStore.fetchConfig()
}

/**
 * Chain 2: Handles Data Prefetching and dependent sequential operations.
 * Flow: Prefetch API -> Sync Store (Token) -> Scrollbar API (needs token) -> Final Trigger
 */
async function processPrefetchChain(
  filterJsonString: string | null,
  priorityId: string,
  sortOrder: GallerySortOrder,
  locate: string | null,
  isolationId: IsolationId,
  route: RouteLocationNormalizedLoadedGeneric
) {
  // 1. Fetch main data (Critical step)
  const prefetchReturn = await prefetch(filterJsonString, priorityId, sortOrder, locate)

  // 2. Sync Store immediately after prefetch returns
  // This updates the Token which fetchScrollbar relies on.
  syncStoreFromPrefetch(prefetchReturn, isolationId)

  // 3. Fetch dependent resources (Scrollbar, Tags, Albums)
  // fetchScrollbar MUST run after syncStoreFromPrefetch because it needs the new Token.
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const dependentPromises: Promise<any>[] = []

  dependentPromises.push(fetchScrollbar(isolationId))


  if (route.meta.baseName !== 'share') {
    const searchFacetStore = useSearchFacetStore()
    if (!searchFacetStore.fetched) {
      dependentPromises.push(searchFacetStore.fetchFacets())
    }

    const albumStore = useAlbumStore('mainId')
    if (!albumStore.fetched) {
      dependentPromises.push(albumStore.fetchAlbums())
    }
  }

  // Wait for all dependent fetches in this chain to complete
  await Promise.all(dependentPromises)

  // 4. Trigger the final row fetch to display the grid
  const prefetchStore = usePrefetchStore(isolationId)
  prefetchStore.updateFetchRowTrigger = !prefetchStore.updateFetchRowTrigger
}

/**
 * Helper to update stores with data from prefetch response.
 */
function syncStoreFromPrefetch(
  prefetchReturn: PrefetchReturn,
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const initializedStore = useInitializedStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const shareStore = useShareStore('mainId')

  const { prefetch, token, resolvedShare } = prefetchReturn

  shareStore.resolvedShare = resolvedShare
  prefetchStore.timestamp = prefetch.timestamp

  prefetchStore.updateVisibleRowTrigger = !prefetchStore.updateVisibleRowTrigger
  prefetchStore.calculateLength(prefetch.dataLength)
  prefetchStore.locateTo = prefetch.locateTo
  tokenStore.timestampToken = token

  initializedStore.initialized = true
}
