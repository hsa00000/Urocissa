<template>
  <v-col
    v-if="metadata && metadata.database"
    id="col-ref"
    class="h-100 d-flex align-center justify-center"
  >
    <img
      :key="index"
      v-if="metadata.database.ext_type === 'image' && imgStore.imgOriginal.get(index)"
      :src="imgStore.imgOriginal.get(index)"
      :style="{
        width: `${metadata.database.width}px`,
        height: `${metadata.database.height}px`,
        maxWidth: '100%',
        maxHeight: '100%',
        objectFit: 'scale-down'
      }"
    />

    <video
      controls
      autoplay
      v-if="metadata.database.ext_type === 'video' && !metadata.database.pending"
      :src="getSrc(hash, false, 'mp4', Cookies.get('jwt')!, undefined)"
      :style="{
        width: `${metadata.database.width}px`,
        height: `${metadata.database.height}px`,
        maxWidth: '100%',
        maxHeight: '100%'
      }"
      inline
      ref="videoRef"
      crossorigin="anonymous"
    >
      >
    </video>
    <v-card
      v-if="metadata.database.ext_type === 'video' && metadata.database.pending"
      class="d-flex align-center justify-start"
      outlined
      style="padding: 16px"
    >
      <v-row align="center" no-gutters>
        <v-col cols="auto" class="d-flex align-center">
          <v-icon size="48" color="warning">mdi-alert-circle-outline</v-icon>
        </v-col>
        <v-col class="text-left pl-4">
          <div>This video is currently being processed.</div>
          <div>Please check back later.</div>
        </v-col>
      </v-row>
    </v-card>
  </v-col>
</template>

<script setup lang="ts">
import { VCol } from 'vuetify/components'
import { useImgStore } from '@/store/imgStore'
import { getSrc } from '@/../config.ts'
import { AbstractData, IsolationId } from '@/script/common/types'
import Cookies from 'js-cookie'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import { ref, watch } from 'vue'

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  metadata: AbstractData
  colWidth: number
  colHeight: number
}>()

const imgStore = useImgStore(props.isolationId)
const currentFrameStore = useCurrentFrameStore(props.isolationId)
const videoRef = ref<HTMLVideoElement | null>(null)

watch(videoRef, () => {
  currentFrameStore.video = videoRef.value
})
</script>
