<script setup lang="ts">
import { computed, onMounted, ref, watchEffect, type Ref } from 'vue'
import { useConstStore } from '@/store/constStore'
import { paddingPixel } from '@/type/constants'
import { getInjectValue } from '@utils/getter'

const props = withDefaults(
  defineProps<{
    topPixel: number
    modifyTopPixel: boolean
    anchorFromBottom?: boolean
  }>(),
  {
    anchorFromBottom: false
  }
)

const constStore = useConstStore('mainId')
const placeholderRef = ref<HTMLElement>()
const windowWidth = getInjectValue<Ref<number>>('windowWidth')
const windowHeight = getInjectValue<Ref<number>>('windowHeight')
const placeholderRefHeight = ref(0)
const placeholderRowRef = ref<HTMLElement[]>([])
const placeholderRowRefHeight = ref(0)

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

const placeholderTransform = computed(() => {
  if (props.anchorFromBottom) {
    // An absolutely positioned element with bottom: 0 starts one element-height above the
    // physical bottom. Add that height back so topPixel remains a distance to its top edge.
    return placeholderRefHeight.value - props.topPixel
  }
  return props.modifyTopPixel
    ? props.topPixel - placeholderRefHeight.value
    : props.topPixel
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

<template>
  <div
    ref="placeholderRef"
    class="w-100 position-absolute"
    :style="{
      top: anchorFromBottom ? 'auto' : '0px',
      bottom: anchorFromBottom ? '0px' : 'auto',
      transform: `translateY(${placeholderTransform}px)`,
      willChange: 'transform'
    }"
  >
    <div
      v-for="index in placeholderRowNumScaled"
      :key="`extra-${index}`"
      class="d-flex flex-wrap"
    >
      <div
        v-for="subindex in placeholderColNum"
        :key="`extra-${subindex}`"
        ref="placeholderRowRef"
        class="bg-placeholder ma-1"
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
