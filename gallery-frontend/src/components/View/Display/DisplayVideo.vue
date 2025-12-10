<template>
  <video
    v-if="tokenReady"
    controls
    :autoplay="enableWatch !== false"
    :src="videoSrc"
    :style="{
      width: `${data.width}px`,
      height: `${data.height}px`,
      maxWidth: '100%',
      maxHeight: '100%'
    }"
    inline
    ref="videoRef"
    crossorigin="anonymous"
  ></video>
</template>

<script setup lang="ts">
import { ref, watch, onBeforeUnmount, onMounted, computed } from 'vue'
import { getSrc } from '@utils/getter'
import { useTokenStore } from '@/store/tokenStore'
import { useShareStore } from '@/store/shareStore'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import type { GalleryVideo, IsolationId } from '@/type/types'

const props = defineProps<{
  isolationId: IsolationId
  data: GalleryVideo
  enableWatch: boolean
}>()

const tokenReady = ref(false)
const tokenStore = useTokenStore(props.isolationId)
const currentFrameStore = useCurrentFrameStore(props.isolationId)
const shareStore = useShareStore('mainId')
const videoRef = ref<HTMLVideoElement | null>(null)

const videoSrc = computed(() =>
  getSrc(
    props.data.id,
    false,
    props.data.ext,
    { albumId: shareStore.albumId || null, shareId: shareStore.shareId || null },
    tokenStore.hashTokenMap.get(props.data.id)
  )
)

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
  await tokenStore.tryRefreshAndStoreTokenToDb(props.data.id)
  tokenReady.value = true
})
</script>
