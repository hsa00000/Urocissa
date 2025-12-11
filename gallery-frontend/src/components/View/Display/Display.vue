<template>
  <div
    id="image-display-col"
    class="h-100 position-relative flex-grow-1 min-w-0 image-col d-flex flex-column"
    :class="{
      'is-overlay-mode': configStore.viewBarOverlay,
      'is-push-mode': !configStore.viewBarOverlay
    }"
  >
    <!-- Overlay toolbar positioned absolutely within the column scope -->
    <ViewBar
      :abstract-data="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />

    <DisplayMobile
      class="flex-grow-1 position-relative view-content"
      v-if="configStore.isMobile"
      :isolation-id="isolationId"
      :hash="hash"
      :index="index"
      :abstract-data="abstractData"
      :previous-hash="previousHash"
      :next-hash="nextHash"
      :previous-page="previousPage"
      :next-page="nextPage"
    />

    <DisplayDesktop
      class="flex-grow-1 position-relative view-content"
      v-if="!configStore.isMobile"
      :isolation-id="isolationId"
      :hash="hash"
      :index="index"
      :abstract-data="abstractData"
      :previous-hash="previousHash"
      :next-hash="nextHash"
      :previous-page="previousPage"
      :next-page="nextPage"
    />
  </div>
</template>

<script setup lang="ts">
import { onUnmounted, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
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
// child display components moved to DisplayMobile / DisplayDesktop
import DisplayMobile from './DisplayMobile.vue'
import DisplayDesktop from './DisplayDesktop.vue'
import delay from 'delay'
import { useConfigStore } from '@/store/configStore'
import { useShareStore } from '@/store/shareStore'
import { useTokenStore } from '@/store/tokenStore'

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
  return nextData?.data.id
})

const previousHash = computed(() => {
  const previousData = dataStore.data.get(props.index - 1)
  return previousData?.data.id
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

  const hash =
    abstractData.data.type === 'album' ? abstractData.data.cover ?? null : abstractData.data.id
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
    if (event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement)
      return
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
</script>

<style scoped>
#image-display-col {
  container-type: size;
  container-name: image-col;
}

/* 推擠模式：ViewBar 會佔據空間，內容為相對定位並填滿剩餘空間 */
.is-push-mode .view-content {
  position: relative;
  flex: 1 1 auto;
  overflow: hidden;
}

/* 覆蓋模式：ViewBar 為絕對定位，不佔空間，內容則鋪滿容器 */
.is-overlay-mode .view-content {
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  overflow: hidden;
}
</style>
