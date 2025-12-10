<template>
  <img
    v-if="imgStore.imgOriginal.get(index)"
    :src="imgStore.imgOriginal.get(index)"
    :style="{
      width: `${data.width}px`,
      height: `${data.height}px`,
      maxWidth: '100%',
      maxHeight: '100%',
      objectFit: 'scale-down'
    }"
  />
</template>

<script setup lang="ts">
import { useImgStore } from '@/store/imgStore'
import type { GalleryImage, IsolationId } from '@/type/types'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  data: GalleryImage
}>()

const imgStore = useImgStore(props.isolationId)
</script>
