<template>
  <img
    :key="index"
    v-if="abstractData?.type === 'image' && imgStore.imgOriginal.get(index)"
    :src="imgStore.imgOriginal.get(index)"
    :style="{
      width: `${abstractData?.width}px`,
      height: `${abstractData?.height}px`,
      maxWidth: '100%',
      maxHeight: '100%',
      objectFit: 'scale-down'
    }"
  />
</template>

<script setup lang="ts">
import { useImgStore } from '@/store/imgStore'
import { EnrichedUnifiedData, IsolationId } from '@type/types'

const props = defineProps<{
  isolationId: IsolationId
  index: number
  abstractData: EnrichedUnifiedData
}>()

const imgStore = useImgStore(props.isolationId)
</script>
