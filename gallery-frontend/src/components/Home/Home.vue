<script setup lang="ts">
import { computed, nextTick, onBeforeUnmount, onMounted, provide, ref, watch } from 'vue'
import { useRoute, type LocationQueryValue } from 'vue-router'
import { useElementSize } from '@vueuse/core'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useWorkerStore } from '@/store/workerStore'
import { useQueueStore } from '@/store/queueStore'
import { usePrefetch } from '@/script/hook/usePrefetch'
import {
  handleScroll,
  type HybridScrollDebugSnapshot,
  type ScrollSyncReason
} from '@/script/hook/useHandleScroll'
import { useInitializeScrollPosition } from '@/script/hook/useInitializeScrollPosition'
import { useImgStore } from '@/store/imgStore'
import Buffer from '@/components/Buffer/Buffer.vue'
import ScrollBar from '@/components/Home/HomeScrollBar.vue'
import { fixedBigRowHeight, layoutBatchNumber } from '@/type/constants'
import { useOffsetStore } from '@/store/offsetStore'
import { useRowStore } from '@/store/rowStore'
import { useLocationStore } from '@/store/locationStore'
import { fetchRowInWorker } from '@/api/fetchRow'
import HomeEmptyCard from '@/components/Home/HomeEmptyCard.vue'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import type { IsolationId } from '@type/types'
import { useCollectionReloadStore } from '@/store/collectionReloadStore'
import { useAlbumStore } from '@/store/albumStore'
import { useConstStore } from '@/store/constStore'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useTokenStore } from '@/store/tokenStore'

const props = defineProps<{
  isolationId: IsolationId
  basicString: string | null
  searchString: LocationQueryValue | LocationQueryValue[] | undefined
}>()

const scrollTopStore = useScrollTopStore(props.isolationId)
const offsetStore = useOffsetStore(props.isolationId)
const rowStore = useRowStore(props.isolationId)
const dataStore = useDataStore(props.isolationId)
const filterStore = useFilterStore(props.isolationId)
const collectionStore = useCollectionStore(props.isolationId)
const prefetchStore = usePrefetchStore(props.isolationId)
const workerStore = useWorkerStore(props.isolationId)
const initializedStore = useInitializedStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const imgStore = useImgStore(props.isolationId)
const locationStore = useLocationStore(props.isolationId)
const optimisticUpateStore = useOptimisticStore(props.isolationId)
const scrollbarStore = useScrollbarStore(props.isolationId)
const tokenStore = useTokenStore(props.isolationId)
// albumStore should not use 'mainId'; otherwise clearAll will be called when the
// props.isolationId component is unmounted.
const albumStore = useAlbumStore(props.isolationId)
const collectionReloadStore = useCollectionReloadStore('mainId')
const constStore = useConstStore('mainId')

const route = useRoute()
const imageContainerRef = ref<HTMLElement | null>(null)
const { width: windowWidth, height: windowHeight } = useElementSize(imageContainerRef)
const clientHeight = ref(0)

provide('imageContainerRef', imageContainerRef)
provide('windowWidth', windowWidth)
provide('windowHeight', windowHeight)

const scrollController = handleScroll(imageContainerRef, windowHeight, props.isolationId)
const {
  effectiveScrollTop,
  generation: scrollGeneration,
  internalWriteCount,
  logicalUpperBound,
  minimumBufferHeight: bufferHeight,
  mode: scrollMode,
  projectionOrigin
} = scrollController

let authoritativeSyncDepth = 0

async function synchronizeLogicalPosition(reason: ScrollSyncReason): Promise<void> {
  authoritativeSyncDepth += 1
  try {
    await scrollController.syncToLogicalPosition(reason)
  } finally {
    // Geometry notifications produced by the same render are already reflected by the sync's
    // post-render P/U measurement and must not be applied a second time.
    await nextTick()
    authoritativeSyncDepth -= 1
  }
}

function requestLogicalPositionSync(reason: ScrollSyncReason): void {
  void synchronizeLogicalPosition(reason).catch((error: unknown) => {
    console.error(`Hybrid scroll ${reason} synchronization failed:`, error)
  })
}

function onGeometryShift(anchorShiftPx: number): void {
  if (
    authoritativeSyncDepth > 0 ||
    prefetchStore.locateTo !== null ||
    locationStore.pendingLocateTarget !== null
  ) {
    return
  }

  void scrollController.reconcileGeometry(anchorShiftPx).catch((error: unknown) => {
    console.error('Hybrid scroll geometry reconciliation failed:', error)
  })
}

function onScroll(): void {
  if (prefetchStore.locateTo === null && locationStore.pendingLocateTarget === null) {
    scrollController.onScroll()
  }
}

function onScrollEnd(): void {
  if (prefetchStore.locateTo === null && locationStore.pendingLocateTarget === null) {
    void scrollController.onScrollEnd().catch((error: unknown) => {
      console.error('Hybrid scroll settlement failed:', error)
    })
  }
}

watch([windowWidth, () => constStore.subRowHeightScale, () => constStore.limitRatio], async () => {
  locationStore.triggerForResize()
  prefetchStore.windowWidth = Math.round(windowWidth.value)
  prefetchStore.clearForResize()
  rowStore.clearForResize()
  offsetStore.clearAll()
  queueStore.clearAll()
  imgStore.clearForResize()
  const locationRowIndex = Math.floor(locationStore.locationIndex / layoutBatchNumber)

  locationStore.anchor = initializedStore.initialized ? locationRowIndex : null
  scrollTopStore.scrollTop = locationRowIndex * fixedBigRowHeight
  await synchronizeLogicalPosition('resize')
  await fetchRowInWorker(locationRowIndex, props.isolationId)
})

let locateWasPending = false
watch([() => prefetchStore.locateTo, () => locationStore.pendingLocateTarget], ([locateTo, target]) => {
  const locateIsPending = locateTo !== null || target !== null
  if (locateWasPending && !locateIsPending) requestLogicalPositionSync('locate')
  locateWasPending = locateIsPending
})

watch(
  () => props.searchString,
  (searchString) => {
    filterStore.searchString = searchString
  },
  { immediate: true }
)

const filterJsonString = computed(() =>
  filterStore.generateFilterJsonString(props.basicString)
)
const reloadTrigger = computed(() =>
  props.isolationId === 'mainId'
    ? collectionReloadStore.mainCollectionReload
    : collectionReloadStore.subCollectionReload
)

const scrollDebugAttributes = computed(() => {
  if (!import.meta.env.DEV) return {}
  return {
    'data-scroll-mode': scrollMode.value,
    'data-scroll-origin': String(projectionOrigin.value),
    'data-scroll-generation': String(scrollGeneration.value),
    'data-scroll-upper': String(logicalUpperBound.value),
    'data-scroll-total-height': String(prefetchStore.totalHeight),
    'data-scroll-internal-writes': String(internalWriteCount.value)
  }
})

function resetCollectionSnapshot(): void {
  if (workerStore.worker !== null || workerStore.imgWorker.length > 0) {
    workerStore.terminateWorker()
  }

  initializedStore.initialized = false
  dataStore.clearAll()
  prefetchStore.timestamp = null
  prefetchStore.calculateLength(0)
  prefetchStore.locateTo = null
  prefetchStore.locateResolution = null
  tokenStore.clearAll()
  queueStore.clearAll()
  collectionStore.clearSelection()
  imgStore.clearAll()
  offsetStore.clearAll()
  rowStore.clearAll()
  scrollbarStore.clearAll()
  locationStore.clearAll()
  optimisticUpateStore.clearAll()
  albumStore.clearAll()
  scrollTopStore.scrollTop = 0
  requestLogicalPositionSync('collection-reset')
}

usePrefetch(filterJsonString, windowWidth, route, props.isolationId, {
  onRequestStart: resetCollectionSnapshot,
  reloadTrigger
})

// Remove the locate query param after the two-step jump fully completes,
// so refreshing won't re-trigger the jump.
// Uses history.replaceState instead of router.replace to avoid changing
// the reactive route object, which would alter routeKey and remount the page.
watch(
  () => locationStore.highlightedIndex,
  (val) => {
    if (val !== null) {
      const url = new URL(window.location.href)
      if (url.searchParams.has('locate')) {
        url.searchParams.delete('locate')
        window.history.replaceState(history.state, '', url)
      }
    }
  }
)

type ScrollDebugElement = HTMLElement & {
  __urocissaScrollDebug?: () => HybridScrollDebugSnapshot
  __urocissaScrollTest?: {
    sync: (logicalTop: number) => Promise<void>
  }
}

let scrollDebugElement: ScrollDebugElement | null = null

watch(
  imageContainerRef,
  (imageContainer, previousImageContainer) => {
    if (previousImageContainer !== null) {
      delete (previousImageContainer as ScrollDebugElement).__urocissaScrollDebug
      delete (previousImageContainer as ScrollDebugElement).__urocissaScrollTest
    }
    scrollDebugElement = imageContainer
    if (import.meta.env.DEV && scrollDebugElement !== null) {
      scrollDebugElement.__urocissaScrollDebug = scrollController.getDebugSnapshot
      scrollDebugElement.__urocissaScrollTest = {
        async sync(logicalTop: number) {
          scrollTopStore.scrollTop = logicalTop
          await synchronizeLogicalPosition('external')
        }
      }
    }
  },
  { flush: 'post' }
)

onMounted(() => {
  useInitializeScrollPosition(
    imageContainerRef,
    clientHeight,
    props.isolationId,
    synchronizeLogicalPosition
  )
})

onBeforeUnmount(() => {
  if (scrollDebugElement !== null) {
    delete scrollDebugElement.__urocissaScrollDebug
    delete scrollDebugElement.__urocissaScrollTest
  }
  scrollController.cancel()
  if (workerStore.worker !== null || workerStore.imgWorker.length > 0) {
    workerStore.terminateWorker()
  }
  initializedStore.initialized = false
  dataStore.clearAll()
  prefetchStore.clearAll()
  tokenStore.clearAll()
  queueStore.clearAll()
  filterStore.searchString = null
  collectionStore.clearSelection()
  imgStore.clearAll()
  offsetStore.clearAll()
  rowStore.clearAll()
  scrollbarStore.clearAll()
  locationStore.clearAll()
  optimisticUpateStore.clearAll()
  albumStore.clearAll()
})
</script>

<template>
  <div class="w-100 h-100 d-flex flex-column">
    <!-- This router-view contains ViewPage.vue. -->
    <router-view></router-view>

    <div class="w-100 flex-grow-0 flex-shrink-0">
      <slot name="home-toolbar"></slot>
    </div>

    <div class="w-100 flex-grow-1 min-h-0 d-flex">
      <div
        id="image-container"
        ref="imageContainerRef"
        v-bind="scrollDebugAttributes"
        class="d-flex flex-wrap position-relative flex-grow-1 min-h-0 h-100 pa-1 pb-2 bg-surface-light overflow-y-scroll"
        @scroll="onScroll"
        @scrollend="onScrollEnd"
      >
        <Buffer
          v-if="initializedStore.initialized && prefetchStore.dataLength > 0"
          :buffer-height="bufferHeight"
          :effective-scroll-top="effectiveScrollTop"
          :isolation-id="props.isolationId"
          :logical-upper-bound="logicalUpperBound"
          :projection-origin="projectionOrigin"
          :scroll-mode="scrollMode"
          @geometry-shift="onGeometryShift"
        />
        <HomeEmptyCard
          v-if="initializedStore.initialized && prefetchStore.dataLength === 0"
          :isolation-id="props.isolationId"
        />
      </div>

      <div class="flex-grow-0 flex-shrink-0 bg-surface-light" style="overflow: visible">
        <ScrollBar
          :isolation-id="props.isolationId"
          @scroll-jump="requestLogicalPositionSync('scrollbar')"
        />
      </div>
    </div>
  </div>
</template>

<style scoped>
#image-container {
  -ms-overflow-style: none;
  scrollbar-width: none;
  overscroll-behavior-y: contain;
  overflow-anchor: none;
}

#image-container::-webkit-scrollbar {
  display: none;
}

img {
  transition: border 0.1s linear;
}
</style>
