<template>
  <video
    v-if="tokenReady"
    controls
    :autoplay="enableWatch !== false"
    :src="
      getSrc(
        hash,
        false,
        'mp4',
        { albumId: shareStore.albumId || null, shareId: shareStore.shareId || null },
        tokenStore.hashTokenMap.get(hash)
      )
    "
    :style="{
      width: `${database.width}px`,
      height: `${database.height}px`,
      maxWidth: '100%',
      maxHeight: '100%'
    }"
    inline
    ref="videoRef"
    crossorigin="anonymous"
  >
    >
  </video>
</template>

<script setup lang="ts">
import { Database, IsolationId } from '@type/types'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { getSrc } from '@utils/getter'
import { useTokenStore } from '@/store/tokenStore'
import { useShareStore } from '@/store/shareStore'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  database: Database
  enableWatch: boolean
}>()

const tokenReady = ref(false)

const tokenStore = useTokenStore(props.isolationId)
const currentFrameStore = useCurrentFrameStore(props.isolationId)
const shareStore = useShareStore('mainId')

const videoRef = ref<HTMLVideoElement | null>(null)

if (props.enableWatch) {
  watch(videoRef, () => {
    currentFrameStore.video = videoRef.value
  })
}

onBeforeUnmount(() => {
  if (currentFrameStore.video === videoRef.value) {
    currentFrameStore.video = null
  }
})

onMounted(async () => {
  await tokenStore.tryRefreshAndStoreTokenToDb(props.database.hash)
  tokenReady.value = true
})
</script>
