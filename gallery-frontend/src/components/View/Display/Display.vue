<template>
  <v-col
    id="image-display-col"
    ref="colRef"
    cols="auto"
    :class="{ 'show-info': constStore.showInfo, 'not-show-info': !constStore.showInfo }"
    class="h-100 flex-grow-1 position-relative d-flex flex-column"
  >
    <!-- Overlay toolbar positioned absolutely within the column scope -->
    <v-row class="flex-grow-0" no-gutters>
      <v-col>
        <ViewBar
          :abstract-data="abstractData"
          :index="index"
          :hash="hash"
          :isolation-id="isolationId"
        />
      </v-col>
    </v-row>

    <!-- Navigation overlays (not grid children) -->
    <v-card
      width="100"
      v-if="!configStore.isMobile && previousHash !== undefined"
      color="transparent"
      class="navigate-left d-flex align-center justify-center h-50"
      style="
        position: absolute;
        left: 0;
        top: 50%;
        transform: translateY(-50%);
        z-index: var(--z-content);
      "
      :to="previousPage"
      replace
    >
      <v-icon>mdi-arrow-left</v-icon>
    </v-card>
    <v-card
      width="100"
      v-if="!configStore.isMobile && nextHash !== undefined"
      color="transparent"
      class="navigate-right d-flex align-center justify-center h-50"
      style="
        position: absolute;
        right: 0;
        top: 50%;
        transform: translateY(-50%);
        z-index: var(--z-content);
      "
      :to="nextPage"
      replace
    >
      <v-icon>mdi-arrow-right</v-icon>
    </v-card>

    <!-- Swiper container for mobile with preview -->
    <v-row v-if="configStore.isMobile" class="flex-grow-1" no-gutters style="min-height: 0">
      <swiper
        id="slider"
        :modules="modules"
        :slides-per-view="1"
        :space-between="10"
        :centered-slides="true"
        :initial-slide="currentSlideIndex"
        :resistance="true"
        :resistance-ratio="0.3"
        :allow-touch-move="canHandleNav()"
        @slide-change="onSlideChange"
        @swiper="onSwiper"
        class="w-100 h-100"
      >
        <!-- Previous slide -->
        <swiper-slide v-if="previousHash !== undefined">
          <div class="w-100 h-100">
            <ViewPageDisplayDatabase
              v-if="
                previousAbstractData && previousAbstractData.database && !configStore.disableImg
              "
              :index="index - 1"
              :hash="previousAbstractData.database.hash"
              :abstract-data="previousAbstractData"
              :col-width="colWidth"
              :col-height="colHeight"
              :isolation-id="isolationId"
            />
            <ViewPageDisplayAlbum
              v-if="previousAbstractData && previousAbstractData.album && !configStore.disableImg"
              :index="index - 1"
              :album="previousAbstractData.album"
              :col-width="colWidth"
              :col-height="colHeight"
            />
          </div>
        </swiper-slide>

        <!-- Current slide -->
        <swiper-slide>
          <div class="w-100 h-100">
            <ViewPageDisplayDatabase
              v-if="abstractData && abstractData.database && !configStore.disableImg"
              :index="index"
              :hash="hash"
              :abstract-data="abstractData"
              :col-width="colWidth"
              :col-height="colHeight"
              :isolation-id="isolationId"
            />
            <ViewPageDisplayAlbum
              v-if="abstractData && abstractData.album && !configStore.disableImg"
              :index="index"
              :album="abstractData.album"
              :col-width="colWidth"
              :col-height="colHeight"
            />
          </div>
        </swiper-slide>

        <!-- Next slide -->
        <swiper-slide v-if="nextHash !== undefined">
          <div class="w-100 h-100">
            <ViewPageDisplayDatabase
              v-if="nextAbstractData && nextAbstractData.database && !configStore.disableImg"
              :index="index + 1"
              :hash="nextAbstractData.database.hash"
              :abstract-data="nextAbstractData"
              :col-width="colWidth"
              :col-height="colHeight"
              :isolation-id="isolationId"
            />
            <ViewPageDisplayAlbum
              v-if="nextAbstractData && nextAbstractData.album && !configStore.disableImg"
              :index="index + 1"
              :album="nextAbstractData.album"
              :col-width="colWidth"
              :col-height="colHeight"
            />
          </div>
        </swiper-slide>
      </swiper>
    </v-row>

    <!-- Desktop version without swiper -->
    <v-row v-if="!configStore.isMobile" no-gutters class="flex-grow-1" style="min-height: 0">
      <ViewPageDisplayDatabase
        v-if="abstractData && !configStore.disableImg"
        :index="index"
        :hash="hash"
        :abstract-data="abstractData"
        :col-width="colWidth"
        :col-height="colHeight"
        :isolation-id="isolationId"
      />
      <ViewPageDisplayAlbum
        v-if="abstractData && abstractData.album && !configStore.disableImg"
        :index="index"
        :album="abstractData.album"
        :col-width="colWidth"
        :col-height="colHeight"
      />
    </v-row>
  </v-col>
</template>

<script setup lang="ts">
import { ref, onUnmounted, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
import { VCol } from 'vuetify/components'
import ViewBar from '@/components/NavBar/ViewBar.vue'
import { useConstStore } from '@/store/constStore'
import { useModalStore } from '@/store/modalStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useImgStore } from '@/store/imgStore'
import { bindActionDispatch } from 'typesafe-agent-events'
import { toImgWorker } from '@/worker/workerApi'
import { useWorkerStore } from '@/store/workerStore'
import { useQueueStore } from '@/store/queueStore'
import { fetchDataInWorker } from '@/api/fetchData'
import { usePrefetchStore } from '@/store/prefetchStore'
import { AbstractData, IsolationId } from '@type/types'
import { useElementSize } from '@vueuse/core'
import ViewPageDisplayDatabase from './DisplayDatabase.vue'
import ViewPageDisplayAlbum from './DisplayAlbum.vue'
import delay from 'delay'
import { useConfigStore } from '@/store/configStore'
import { useShareStore } from '@/store/shareStore'
import { useTokenStore } from '@/store/tokenStore'
import { Swiper, SwiperSlide } from 'swiper/vue'
import { Manipulation } from 'swiper/modules'
import 'swiper/css'
import 'swiper/css/manipulation'
import type { Swiper as SwiperType } from 'swiper'

const colRef = ref<InstanceType<typeof VCol> | null>(null)
const { width: colWidth, height: colHeight } = useElementSize(colRef)

// Swiper configuration
const modules = [Manipulation]
const swiperInstance = ref<SwiperType | null>(null)

// Calculate current slide index based on available slides
const currentSlideIndex = computed(() => {
  return previousHash.value !== undefined ? 1 : 0
})

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData | undefined
}>()

const configStore = useConfigStore(props.isolationId)
const prefetchStore = usePrefetchStore(props.isolationId)
const workerStore = useWorkerStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const imgStore = useImgStore(props.isolationId)
const initializedStore = useInitializedStore(props.isolationId)
const tokenStore = useTokenStore(props.isolationId)
const modalStore = useModalStore('mainId')
const constStore = useConstStore('mainId')
const shareStore = useShareStore('mainId')
const dataStore = useDataStore(props.isolationId)
const route = useRoute()
const router = useRouter()

const nextHash = computed(() => {
  const nextData = dataStore.data.get(props.index + 1)
  if (nextData?.database) return nextData.database.hash
  if (nextData?.album) return nextData.album.id
  return undefined
})

const previousHash = computed(() => {
  const previousData = dataStore.data.get(props.index - 1)
  if (previousData?.database) return previousData.database.hash
  if (previousData?.album) return previousData.album.id
  return undefined
})

const nextAbstractData = computed(() => {
  return dataStore.data.get(props.index + 1)
})

const previousAbstractData = computed(() => {
  return dataStore.data.get(props.index - 1)
})

const nextPage = computed(() => {
  if (nextHash.value === undefined) return undefined
  if (route.meta.level === 2) {
    const updatedParams = { ...route.params, hash: nextHash.value }
    return { ...route, params: updatedParams }
  } else if (route.meta.level === 4) {
    const updatedParams = { ...route.params, subhash: nextHash.value }
    return { ...route, params: updatedParams }
  }
  return undefined
})

const previousPage = computed(() => {
  if (previousHash.value === undefined) return undefined
  if (route.meta.level === 2) {
    const updatedParams = { ...route.params, hash: previousHash.value }
    return { ...route, params: updatedParams }
  } else if (route.meta.level === 4) {
    const updatedParams = { ...route.params, subhash: previousHash.value }
    return { ...route, params: updatedParams }
  }
  return undefined
})

const workerIndex = computed(() => props.index % constStore.concurrencyNumber)

const postToWorker = bindActionDispatch(toImgWorker, (action) => {
  const worker = workerStore.imgWorker[workerIndex.value]
  if (worker) {
    worker.postMessage(action)
  } else {
    throw new Error(`Worker not found for index: ${workerIndex.value}`)
  }
})

async function checkAndFetch(index: number): Promise<boolean> {
  if (imgStore.imgOriginal.has(index)) return true
  if (queueStore.original.has(index)) return false

  const abstractData = dataStore.data.get(index)
  if (!abstractData) return false

  queueStore.original.add(index)

  const hash = abstractData.database?.hash ?? abstractData.album?.cover
  if (hash == null) return false

  await tokenStore.refreshTimestampTokenIfExpired()
  await tokenStore.refreshHashTokenIfExpired(hash)

  const timestampToken = tokenStore.timestampToken
  if (timestampToken === null) {
    console.error('timestampToken is null after refresh')
    return false
  }

  const hashToken = tokenStore.hashTokenMap.get(hash)
  if (hashToken === undefined) {
    console.error(`hashToken is undefined after refresh for hash: ${hash}`)
    return false
  }

  postToWorker.processImage({
    index,
    hash,
    devicePixelRatio: window.devicePixelRatio,
    albumId: shareStore.albumId,
    shareId: shareStore.shareId,
    timestampToken,
    hashToken
  })

  return false
}

async function prefetch(index: number, isolationId: IsolationId) {
  if (configStore.disableImg) return

  for (let i = 1; i <= 10; i++) {
    const nextIndex = index + i
    const nextAbstractData = dataStore.data.get(nextIndex)
    if (nextAbstractData) {
      await checkAndFetch(nextIndex)
    } else if (nextIndex <= prefetchStore.dataLength - 1) {
      await fetchDataInWorker('single', nextIndex, isolationId)
    }

    const previousIndex = index - i
    const previousAbstractData = dataStore.data.get(previousIndex)
    if (previousAbstractData) {
      await checkAndFetch(previousIndex)
    } else if (previousIndex >= 0) {
      await fetchDataInWorker('single', previousIndex, isolationId)
    }

    await delay(100)
  }
}

watch(
  [() => props.index, () => initializedStore.initialized],
  async () => {
    if (initializedStore.initialized) {
      if (configStore.disableImg) return
      await checkAndFetch(props.index)
      await prefetch(props.index, props.isolationId)
    }
  },
  { immediate: true }
)

const handleKeyDown = (event: KeyboardEvent) => {
  if (
    (route.meta.level === 2 && props.isolationId === 'mainId') ||
    (route.meta.level === 4 && props.isolationId === 'subId')
  ) {
    if (modalStore.showEditTagsModal) return
    if (event.key === 'ArrowRight' && nextPage.value) {
      router.replace(nextPage.value).catch((error: unknown) => {
        console.error('Navigation Error:', error)
      })
    } else if (event.key === 'ArrowLeft' && previousPage.value) {
      router.replace(previousPage.value).catch((error: unknown) => {
        console.error('Navigation Error:', error)
      })
    }
  }
}

window.addEventListener('keydown', handleKeyDown)

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeyDown)
})

/** -------------------------
 *  Swiper navigation handlers
 *  -------------------------
 */
function canHandleNav(): boolean {
  return (
    configStore.isMobile &&
    ((route.meta.level === 2 && props.isolationId === 'mainId') ||
      (route.meta.level === 4 && props.isolationId === 'subId')) &&
    !modalStore.showEditTagsModal
  )
}

function onSwiper(swiper: SwiperType) {
  swiperInstance.value = swiper
}

function onSlideChange(swiper: SwiperType) {
  if (!canHandleNav()) return

  const currentIndex = swiper.activeIndex
  const hasPrevious = previousHash.value !== undefined
  const hasNext = nextHash.value !== undefined

  // Calculate expected indices based on available slides
  if (hasPrevious) {
    // Layout: [Previous, Current, Next?]
    if (currentIndex === 0 && previousPage.value) {
      // Moved to previous slide
      router.replace(previousPage.value).catch((error: unknown) => {
        console.error('Navigation Error:', error)
      })
    } else if (currentIndex === 2 && hasNext && nextPage.value) {
      // Moved to next slide
      router.replace(nextPage.value).catch((error: unknown) => {
        console.error('Navigation Error:', error)
      })
    }
  } else {
    // Layout: [Current, Next?]
    if (currentIndex === 1 && hasNext && nextPage.value) {
      // Moved to next slide
      router.replace(nextPage.value).catch((error: unknown) => {
        console.error('Navigation Error:', error)
      })
    }
  }
}

// Reset swiper to center when props.index changes
watch(
  () => props.index,
  () => {
    if (swiperInstance.value && configStore.isMobile) {
      // Reset to center slide without animation
      swiperInstance.value.slideTo(currentSlideIndex.value, 0)
    }
  }
)
</script>

<style scoped>
.show-info {
  width: calc(100% - 360px);
}

@media (max-width: 720px) {
  .show-info {
    display: none;
  }
}

.not-show-info {
  width: 100%;
}
</style>
