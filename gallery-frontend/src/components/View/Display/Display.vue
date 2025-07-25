<template>
  <v-col
    id="image-display-col"
    ref="colRef"
    cols="auto"
    :class="{ 'show-info': infoStore.showInfo, 'not-show-info': !infoStore.showInfo }"
    class="h-100"
  >
    <v-row no-gutters class="h-100 position-relative">
      <ViewBar :metadata="metadata" :index="index" :hash="hash" :isolation-id="isolationId" />
      <ViewPageDisplayDatabase
        v-if="metadata && !configStore.disableImg"
        :index="index"
        :hash="hash"
        :metadata="metadata"
        :col-width="colWidth"
        :col-height="colHeight"
        :isolation-id="isolationId"
      />
      <ViewPageDisplayAlbum
        v-if="metadata && metadata.album && !configStore.disableImg"
        :index="index"
        :album="metadata.album"
        :col-width="colWidth"
        :col-height="colHeight"
      />
      <v-card
        v-if="previousHash !== undefined"
        color="transparent"
        class="navigate-left h-100 d-flex align-center justify-center"
        style="position: absolute; left: 0"
        :to="previousPage"
      >
        <v-icon>mdi-arrow-left</v-icon>
      </v-card>
      <v-card
        v-if="nextHash !== undefined"
        color="transparent"
        class="navigate-right h-100 d-flex align-center justify-center"
        style="position: absolute; right: 0"
        :to="nextPage"
      >
        <v-icon>mdi-arrow-right</v-icon>
      </v-card>
    </v-row>
  </v-col>
</template>

<script setup lang="ts">
import { ref, onUnmounted, computed, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
import { VCol } from 'vuetify/components'
import ViewBar from '@/components/NavBar/ViewBar.vue'
import { useInfoStore } from '@/store/infoStore'
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

const colRef = ref<InstanceType<typeof VCol> | null>(null)
const { width: colWidth, height: colHeight } = useElementSize(colRef)

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  metadata: AbstractData | undefined
}>()

const configStore = useConfigStore(props.isolationId)
const prefetchStore = usePrefetchStore(props.isolationId)
const workerStore = useWorkerStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const imgStore = useImgStore(props.isolationId)
const initializedStore = useInitializedStore(props.isolationId)
const tokenStore = useTokenStore(props.isolationId)
const modalStore = useModalStore('mainId')
const infoStore = useInfoStore('mainId')
const shareStore = useShareStore('mainId')
const dataStore = useDataStore(props.isolationId)
const route = useRoute()
const router = useRouter()

const nextHash = computed(() => {
  const nextData = dataStore.data.get(props.index + 1)
  if (nextData?.database) {
    return nextData.database.hash
  } else if (nextData?.album) {
    return nextData.album.id
  } else {
    return undefined
  }
})

const previousHash = computed(() => {
  const previousData = dataStore.data.get(props.index - 1)
  if (previousData?.database) {
    return previousData.database.hash
  } else if (previousData?.album) {
    return previousData.album.id
  } else {
    return undefined
  }
})

const nextPage = computed(() => {
  if (nextHash.value === undefined) {
    return undefined
  }
  if (!route.meta.isReadPage) {
    const updatedParams = { ...route.params, hash: nextHash.value }
    return { ...route, params: updatedParams }
  } else if (props.isolationId === 'subId') {
    const updatedParams = { ...route.params, subhash: nextHash.value }
    return { ...route, params: updatedParams }
  } else {
    return undefined
  }
})

const previousPage = computed(() => {
  if (previousHash.value === undefined) {
    return undefined
  }
  if (!route.meta.isReadPage) {
    const updatedParams = { ...route.params, hash: previousHash.value }
    return { ...route, params: updatedParams }
  } else if (props.isolationId === 'subId') {
    const updatedParams = { ...route.params, subhash: previousHash.value }
    return { ...route, params: updatedParams }
  } else {
    return undefined
  }
})

const workerIndex = computed(() => {
  return props.index % workerStore.concurrencyNumber
})

const postToWorker = bindActionDispatch(toImgWorker, (action) => {
  const worker = workerStore.imgWorker[workerIndex.value]
  if (worker) {
    worker.postMessage(action)
  } else {
    throw new Error(`Worker not found for index: ${workerIndex.value}`)
  }
})

async function checkAndFetch(index: number): Promise<boolean> {
  // If the image is already fetched, return true
  if (imgStore.imgOriginal.has(index)) {
    return true
  }

  // If the image is already in the queue, fetching is not done
  if (queueStore.original.has(index)) {
    return false
  }

  // Retrieve the abstract data for the given index
  const abstractData = dataStore.data.get(index)
  if (!abstractData) {
    return false
  }

  // Add the index to the fetch queue
  queueStore.original.add(index)

  // Determine the hash from database or album cover
  const hash = abstractData.database?.hash ?? abstractData.album?.cover

  if (hash == null) {
    return false
  }

  // Refresh tokens before using them
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

  // If a valid hash exists, initiate the image processing
  postToWorker.processImage({
    index,
    hash,
    devicePixelRatio: window.devicePixelRatio,
    albumId: shareStore.albumId,
    shareId: shareStore.shareId,
    timestampToken,
    hashToken
  })

  // Fetching has been initiated but not completed
  return false
}

async function prefetch(index: number, isolationId: IsolationId) {
  if (configStore.disableImg) {
    return
  }
  for (let i = 1; i <= 10; i++) {
    const nextIndex = index + i
    const nextAbstractData = dataStore.data.get(nextIndex)
    if (nextAbstractData) {
      if (nextAbstractData.database && nextAbstractData.database.ext_type === 'image') {
        await checkAndFetch(nextIndex)
      } else {
        // is album
        await checkAndFetch(nextIndex)
      }
    } else {
      // dataStore.data.get(nextIndex) is undefined then fetch that data
      if (nextIndex <= prefetchStore.dataLength - 1) {
        await fetchDataInWorker('single', nextIndex, isolationId)
      }
    }

    const previousIndex = index - i
    const previousAbstractData = dataStore.data.get(previousIndex)
    if (previousAbstractData) {
      if (previousAbstractData.database && previousAbstractData.database.ext_type === 'image') {
        await checkAndFetch(previousIndex)
      } else {
        // is album
        await checkAndFetch(previousIndex)
      }
    } else {
      // dataStore.data.get(previousIndex) is undefined then fetch that data
      if (previousIndex >= 0) {
        await fetchDataInWorker('single', previousIndex, isolationId)
      }
    }

    await delay(100)
  }
}

watch(
  [() => props.index, () => initializedStore.initialized],
  async () => {
    if (initializedStore.initialized) {
      if (configStore.disableImg) {
        return
      }
      await checkAndFetch(props.index)
      // Prefetch next and previous 10 hashes if they exist
      await prefetch(props.index, props.isolationId)
    }
  },
  { immediate: true }
)

const handleKeyDown = (event: KeyboardEvent) => {
  if (
    (!route.meta.isReadPage && props.isolationId === 'mainId') ||
    (route.meta.isReadPage && props.isolationId === 'subId')
    // prevent two ViewPageDisplay triggered simultaneously
  ) {
    if (modalStore.showEditTagsModal) {
      return
    }
    if (event.key === 'ArrowRight' && nextPage.value) {
      router
        .push(nextPage.value)
        .then(() => ({}))
        .catch((error: unknown) => {
          console.error('Navigation Error:', error)
        })
    } else if (event.key === 'ArrowLeft' && previousPage.value) {
      router
        .push(previousPage.value)
        .then(() => ({}))
        .catch((error: unknown) => {
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
.my-toolbar {
  z-index: 1;
  background: linear-gradient(
    to bottom,
    rgba(0, 0, 0, 0.5) 0%,
    rgba(0, 0, 0, 0.25) 50%,
    rgba(0, 0, 0, 0) 100%
  );
}

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
