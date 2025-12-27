<template>
  <div
    class="w-100 position-absolute"
    ref="placeholderRef"
    :style="{
      top: `${modifyTopPixel ? topPixel - placeholderRefHeight : topPixel}px`
    }"
  >
    <div class="d-flex flex-wrap" v-for="index in placeholderRowNumScaled" :key="`extra-${index}`">
      <div
        v-for="subindex in placeholderColNum"
        :class="`${placeholderBgClass} ma-1`"
        ref="placeholderRowRef"
        :key="`extra-${subindex}`"
        :style="{
          flexGrow: '1',
          position: 'relative',
          width: `${placeholderWidth}px`
        }"
      >
        <i
          class="d-block"
          :style="{ paddingBottom: `${(placeHolderHeight / placeholderWidth) * 100}%` }"
        ></i>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useConstStore } from '@/store/constStore'
import { paddingPixel } from '@/type/constants'
import { getInjectValue } from '@utils/getter'
import { computed, onMounted, Ref, ref, watchEffect } from 'vue'
import { useThemeClasses } from '@/style/useThemeClasses'

const constStore = useConstStore('mainId')
const { placeholderBgClass } = useThemeClasses()

const placeholderRef = ref<HTMLElement>()
const windowWidth = getInjectValue<Ref<number>>('windowWidth')
const windowHeight = getInjectValue<Ref<number>>('windowHeight')

const placeholderRefHeight = ref(0)
const placeholderRowRef = ref<HTMLElement[]>([])
const placeholderRowRefHeight = ref(0)

defineProps<{
  topPixel: number
  modifyTopPixel: boolean
}>()

const placeholderWidth = computed(() => {
  return windowWidth.value !== 0
    ? Math.min(constStore.subRowHeightScale, windowWidth.value) - 2 * paddingPixel
    : constStore.subRowHeightScale
})

const placeholderWidthWithPadding = computed(() => {
  return placeholderWidth.value + 2 * paddingPixel
})

const placeHolderHeight = computed(() => {
  return (placeholderWidth.value * 2) / 3
})

const placeholderColNum = computed(() => {
  return Math.floor(windowWidth.value / placeholderWidthWithPadding.value)
})

const placeholderRowNum = computed(() => {
  return Math.ceil(windowHeight.value / placeHolderHeight.value)
})

const placeholderRowNumScaled = computed(() => {
  return Math.ceil(2 * placeholderRowNum.value)
})

onMounted(() => {
  watchEffect(() => {
    if (placeholderRef.value && placeholderRef.value.clientHeight > 0) {
      placeholderRefHeight.value = placeholderRef.value.clientHeight
    }
  })
  watchEffect(() => {
    if (placeholderRowRef.value[0] !== undefined && placeholderRowRef.value[0].clientHeight > 0) {
      placeholderRowRefHeight.value = placeholderRowRef.value[0].clientHeight
    }
  })
})

defineExpose({
  placeholderRowRefHeight
})
</script>
