<template>
  <!-- This bar is used inside album reading page -->
  <slot name="reading-bar"> </slot>
  <!-- This router-view contains the ViewPage.vue -->
  <router-view></router-view>
  <ScrollBar v-if="imageContainerRef" :isolation-id="props.isolationId" />
  <div
    id="image-container"
    ref="imageContainerRef"
    class="d-flex flex-wrap position-relative pa-1 pb-2 h-100 bg-grey-darken-3"
    :style="{
      width: `calc(100% - ${scrollBarWidth}px)`
    }"
    :class="stopScroll ? 'overflow-y-hidden' : 'overflow-y-scroll'"
    @scroll="
      // If prefetchStore.locateTo triggers initializeScrollPosition, prevent the user from triggering the scrolling function.
      prefetchStore.totalHeight - windowHeight > 0 && prefetchStore.locateTo === null
        ? throttledHandleScroll()
        : () => {}
    "
  >
    <Buffer
      v-if="initializedStore.initialized && prefetchStore.dataLength > 0"
      :buffer-height="bufferHeight"
      :isolation-id="props.isolationId"
    />
    <HomeEmptyCard
      v-if="initializedStore.initialized && prefetchStore.dataLength === 0"
      :isolation-id="props.isolationId"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * Home.vue component handles user scrolling using a custom virtual scrolling implementation.
 *
 * This implementation addresses a limitation of standard virtual scrolling, where the DOM height can reach
 * the browser limit of 33,554,400px. To prevent this issue, a subcomponent called `Buffer` is used with a large height.
 * This setup allows `HomePage.vue` to remain scrollable without hitting the top or bottom prematurely.
 *
 * The scrolling logic is managed by the `throttledHandleScroll()` function. When the user scrolls, this function
 * adjusts the scroll position by updating `scrollTop.value` and restores the value of `imageContainerRef.scrollTop`.
 * This approach enables virtual scrolling to handle heights up to 2^32-1 px or 2^64-1 px in 64-bit Chrome.
 */
import { ref, onMounted, computed, provide, onBeforeUnmount, watch } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useWorkerStore } from '@/store/workerStore'
import { useQueueStore } from '@/store/queueStore'
import { useRoute } from 'vue-router'
import { useElementSize } from '@vueuse/core'
import { prefetch } from '@/components/hook/usePrefetch'
import { handleScroll } from '@/components/hook/useHandleScroll'
import { useInitializeScrollPosition } from '@/components/hook/useInitializeScrollPosition'
import { useImgStore } from '@/store/imgStore'
import MobileDetect from 'mobile-detect'
import Buffer from '@/components/Home/Buffer/Buffer.vue'
import ScrollBar from '@/components/Home/HomeScrollBar.vue'
import '@/style/HomePage.css'
import { layoutBatchNumber, scrollBarWidth } from '@/script/common/constants'
import { useOffsetStore } from '@/store/offsetStore'
import { useRowStore } from '@/store/rowStore'
import { debounce } from 'lodash'
import { useLocationStore } from '@/store/locationStore'
import { fetchRowInWorker } from '@/script/inWorker/fetchRowInWorker'
import HomeEmptyCard from './HomeEmptyCard.vue'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { IsolationId } from '@/script/common/types'

const props = defineProps<{
  isolationId: IsolationId
  tempMode: string | null
  title: string | null
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

const route = useRoute()
const imageContainerRef = ref<HTMLElement | null>(null)
const { width: windowWidth, height: windowHeight } = useElementSize(imageContainerRef)
const clientHeight = ref<number>(0)

const lastScrollTop = ref(0)
const stopScroll = ref(false)

const md = new MobileDetect(window.navigator.userAgent)
const mobile = md.mobile()

provide('imageContainerRef', imageContainerRef)
provide('windowWidth', windowWidth)
provide('windowHeight', windowHeight)
provide('mobile', mobile)

const throttledHandleScroll = handleScroll(
  imageContainerRef,
  lastScrollTop,
  mobile,
  stopScroll,
  windowHeight,
  props.isolationId
)

watch(windowWidth, () => {
  // Handles browser resizing.
  locationStore.triggerForResize()
  prefetchStore.windowWidth = Math.round(windowWidth.value)
  prefetchStore.clearForResize()
  rowStore.clearForResize()
  offsetStore.clearAll()
  queueStore.clearAll()
  imgStore.clearForResize()
  resizeDebounce()
})

const resizeDebounce = debounce(() => {
  const locationRowIndex = Math.floor(locationStore.locationIndex / layoutBatchNumber)
  scrollTopStore.scrollTop = locationRowIndex * 2400
  fetchRowInWorker(locationRowIndex, props.isolationId)
}, 100)

const bufferHeight = computed(() => {
  if (prefetchStore.totalHeight <= windowHeight.value) {
    return 0
  } else {
    return 600000
    // A large value to enable scrolling within the imageContainer without reaching the top or bottom prematurely.
    // This value must be a multiple of 3 to avoid pixel misalignment when dividing bufferHeight by 3 (in useInitializeScrollPosition.ts).
  }
})

onMounted(() => {
  if (props.tempMode === null) {
    filterStore.handleFilterString(route)
    filterStore.handleBasicString(route, props.isolationId)

    prefetch(filterStore.generateFilterJsonString(), windowWidth, route, props.isolationId)
  } else {
    prefetch(props.tempMode, windowWidth, route, props.isolationId)
  }
  useInitializeScrollPosition(
    imageContainerRef,
    bufferHeight,
    lastScrollTop,
    clientHeight,
    windowWidth,
    props.isolationId
  )
})

onBeforeUnmount(() => {
  workerStore.terminateWorker()
  initializedStore.initialized = false
  dataStore.clearAll()
  prefetchStore.clearAll()
  queueStore.clearAll()
  filterStore.basicString = null
  filterStore.filterString = null
  collectionStore.editModeCollection.clear()
  imgStore.clearAll()
  offsetStore.clearAll()
  rowStore.clearAll()
  locationStore.clearAll()
  optimisticUpateStore.clearAll()
})
</script>

<style scoped>
#image-container {
  -ms-overflow-style: none;
  /* IE and Edge */
  scrollbar-width: none;
  /* Firefox */
}

#image-container::-webkit-scrollbar {
  display: none;
  /* Chrome, Safari, and Opera */
}

img {
  transition: border 0.1s linear;
}
</style>
