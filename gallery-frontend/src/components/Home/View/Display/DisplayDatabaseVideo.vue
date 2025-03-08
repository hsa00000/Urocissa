<template>
  <video
    controls
    autoplay
    :src="getSrc(hash, false, 'mp4', Cookies.get('jwt')!, undefined)"
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
import { Database, IsolationId } from '@/script/common/types'
import Cookies from 'js-cookie'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import { ref, watch } from 'vue'
import { getSrc } from '@/../config'
const props = defineProps<{
  isolationId: IsolationId
  hash: string
  database: Database
}>()

const currentFrameStore = useCurrentFrameStore(props.isolationId)
const videoRef = ref<HTMLVideoElement | null>(null)

watch(videoRef, () => {
  currentFrameStore.video = videoRef.value
})
</script>
