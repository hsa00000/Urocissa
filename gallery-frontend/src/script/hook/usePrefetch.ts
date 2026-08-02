
import { watchDebounced } from '@vueuse/core'
import { computed, onScopeDispose, watch, type Ref, type WatchSource } from 'vue'
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
import { consumeInitialMainLocateOverride } from '@/route/initialRouteLocate'

interface UsePrefetchOptions {
  onRequestStart?: () => void
  reloadTrigger?: Readonly<Ref<unknown>>
}

export function usePrefetch(
  filterJsonString: Readonly<Ref<string | null>>,
  windowWidth: Ref<number>,
  route: RouteLocationNormalizedLoadedGeneric,
  isolationId: IsolationId,
  options: UsePrefetchOptions = {}
) {
  const queryAppliesToCollection = computed(
    () =>
      (isolationId === 'subId' && route.meta.level >= 3) ||
      (isolationId === 'mainId' && route.meta.level < 3)
  )
  const priorityId = computed(() =>
    queryAppliesToCollection.value && typeof route.query.priority_id === 'string'
      ? route.query.priority_id
      : ''
  )
  const sortOrder = computed<GallerySortOrder>(() =>
    queryAppliesToCollection.value
      ? parseGallerySortOrder(route.query.sort)
      : 'descending'
  )
  const queryLocate = computed(() =>
    queryAppliesToCollection.value && typeof route.query.locate === 'string'
      ? route.query.locate
      : null
  )

  // Watch the values that define the collection request, not an aggregate
  // object. Vue compares multi-source watch values individually, so moving
  // between nested route levels cannot reload an unchanged collection.
  const requestSources: [
    WatchSource<boolean>,
    WatchSource<string | null>,
    WatchSource<string>,
    WatchSource<GallerySortOrder>,
    WatchSource<string | null>,
    WatchSource<unknown>
  ] = [
    () => windowWidth.value > 0,
    () => filterJsonString.value,
    priorityId,
    sortOrder,
    queryLocate,
    () => options.reloadTrigger?.value
  ]

  let generation = 0
  let disposed = false
  const stopGenerationWatcher = watch(
    requestSources,
    () => {
      generation += 1
    },
    { flush: 'sync' }
  )

  const stopWatcher = watchDebounced(
    requestSources,
    async ([widthReady, currentFilter, currentPriorityId, currentSortOrder, currentQueryLocate]) => {
      if (widthReady) {
        const requestGeneration = generation
        let locate: string | null = null

        // add locate to query string if user enter view page directly
        const initialMainLocate =
          isolationId === 'mainId' ? consumeInitialMainLocateOverride() : null

        if (initialMainLocate !== null) {
          locate = initialMainLocate
        } else if (
          isolationId === 'subId' &&
          route.meta.level === 4 &&
          typeof route.params.subhash === 'string'
        ) {
          locate = route.params.subhash
        } else if (isolationId === 'mainId' && typeof route.params.hash === 'string') {
          locate = route.params.hash
        } else if (currentQueryLocate !== null) {
          locate = currentQueryLocate
        }

        // Invalidate the visible collection before waiting on the network so
        // query changes immediately enter the existing loading state.
        options.onRequestStart?.()

        // Parallel Execution: Run Config chain and Prefetch chain simultaneously
        await Promise.all([
          processConfigChain(isolationId),
          processPrefetchChain(
            currentFilter,
            currentPriorityId,
            currentSortOrder,
            locate,
            isolationId,
            route,
            () => !disposed && requestGeneration === generation
          )
        ])
      }
    },
    { immediate: true, debounce: 75, maxWait: 1000 }
  )

  onScopeDispose(() => {
    disposed = true
    generation += 1
    stopGenerationWatcher()
    stopWatcher()
  })
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
  route: RouteLocationNormalizedLoadedGeneric,
  isCurrent: () => boolean
) {
  // 1. Fetch main data (Critical step)
  const prefetchReturn = await prefetch(filterJsonString, priorityId, sortOrder, locate)

  // Query-only navigations can start a newer request without unmounting the
  // reader. Never let an older response replace that newer snapshot.
  if (!isCurrent()) return

  // 2. Sync Store immediately after prefetch returns
  // This updates the Token which fetchScrollbar relies on.
  syncStoreFromPrefetch(prefetchReturn, isolationId)

  // Rows can render as soon as the snapshot and token exist.
  const prefetchStore = usePrefetchStore(isolationId)
  prefetchStore.updateFetchRowTrigger = !prefetchStore.updateFetchRowTrigger

  // 3. Fetch dependent resources (Scrollbar, Tags, Albums) in the background.
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

  void Promise.allSettled(dependentPromises)
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
