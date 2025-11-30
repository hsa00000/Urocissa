<template>
  <img
    :key="index"
    v-if="abstractData?.database?.extType === 'image' && imgStore.imgOriginal.get(index)"
    :src="imgStore.imgOriginal.get(index)"
    :style="{
      width: `${abstractData?.database?.width}px`,
      height: `${abstractData?.database?.height}px`,
      maxWidth: '100%',
      maxHeight: '100%',
      objectFit: 'scale-down'
    }"
  />
</template>

<script setup lang="ts">
import { useImgStore } from '@/store/imgStore'
import { AbstractData, IsolationId } from '@type/types'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  abstractData: AbstractData
}>()

const imgStore = useImgStore(props.isolationId)
</script>
