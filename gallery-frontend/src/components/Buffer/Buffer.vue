<script setup lang="ts">
/**
 * Buffer renders the existing worker-owned row geometry into the controller's current physical
 * projection. It never mutates the committed logical scroll position. Top and compensated modes
 * project from the physical top; native-bottom projects from the physical bottom so browser
 * layout quantization never receives the full logical document height in a transform.
 */
import {
  computed,
  provide,
  ref,
  watch,
  type ComponentPublicInstance,
  type CSSProperties,
  type Ref
} from 'vue'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useFetchImgs } from '@/script/hook/useFetchImgs'
import { useUpdateVisibleRows } from '@/script/hook/useUpdateVisibleRows'
import { useFetchRows } from '@/script/hook/useFetchRows'
import { batchNumber, paddingPixel } from '@/type/constants'
import BufferPlaceholder from '@/components/Buffer/BufferPlaceholder.vue'
import RowBlock from '@/components/Buffer/BufferRowBlock.vue'
import { getInjectValue } from '@utils/getter'
import type { IsolationId } from '@type/types'
import { useRowStore } from '@/store/rowStore'
import { scrollActivityKey, useScrollActivity } from '@/script/hook/useScrollActivity'
import type { HybridScrollMode } from '@/script/hook/useHandleScroll'
import {
  projectRelativeTop,
  projectVirtualBottom,
  projectVirtualTop,
  resolvePhysicalBufferHeight
} from '@/script/utils/rowOffset'

const props = defineProps<{
  isolationId: IsolationId
  bufferHeight: number
  effectiveScrollTop: number
  scrollMode: HybridScrollMode
  projectionOrigin: number
  logicalUpperBound: number
}>()

const emit = defineEmits<{
  geometryShift: [anchorShiftPx: number]
}>()

const prefetchStore = usePrefetchStore(props.isolationId)
const rowStore = useRowStore(props.isolationId)
const scrollActivity = useScrollActivity(() => props.effectiveScrollTop)
provide(scrollActivityKey, scrollActivity)

const windowWidth = getInjectValue<Ref<number>>('windowWidth')
const windowHeight = getInjectValue<Ref<number>>('windowHeight')
const imageContainerRef = getInjectValue<Ref<HTMLElement | null>>('imageContainerRef')

type BufferPlaceholderInstance = ComponentPublicInstance<{
  placeholderRowRefHeight: number
}>

const placeholderNoneRef = ref<BufferPlaceholderInstance | null>(null)
const lastRowBottom = ref(0)

const placeholderNoneRowRefHeight = computed(() =>
  placeholderNoneRef.value ? placeholderNoneRef.value.placeholderRowRefHeight : 0
)
const physicalBufferHeight = computed(() =>
  resolvePhysicalBufferHeight(props.bufferHeight, prefetchStore.totalHeight)
)
const visibleRowsLength = computed(() => visibleRows.value.length)
const visibleRowsLogicalTop = computed(() => {
  const firstRow = visibleRows.value[0]
  return firstRow === undefined ? 0 : firstRow.topPixelAccumulated + firstRow.offset
})
const startHeight = computed(() => props.effectiveScrollTop)
const endHeight = computed(() => props.effectiveScrollTop + windowHeight.value)

const { visibleRows } = useUpdateVisibleRows(
  imageContainerRef,
  startHeight,
  endHeight,
  lastRowBottom,
  props.isolationId,
  (anchorShiftPx) => {
    emit('geometryShift', anchorShiftPx)
  }
)

const visibleRowsProjectionStyle = computed<CSSProperties>(() => {
  if (props.scrollMode === 'native-bottom') {
    return {
      top: 'auto',
      bottom: '0px',
      height: '0px',
      transform: `translateY(${-projectVirtualBottom(
        visibleRowsLogicalTop.value,
        props.logicalUpperBound,
        windowHeight.value
      )}px)`,
      willChange: 'transform'
    }
  }

  return {
    top: '0px',
    bottom: 'auto',
    height: '0px',
    transform: `translateY(${projectVirtualTop(
      visibleRowsLogicalTop.value,
      props.projectionOrigin
    )}px)`,
    willChange: 'transform'
  }
})

const placeholderBottomTop = computed(() => {
  const lastRow = visibleRows.value[visibleRows.value.length - 1]
  if (lastRow === undefined) return 0
  return projectRelativeTop(
    lastRow.topPixelAccumulated + lastRow.offset + lastRow.rowHeight,
    visibleRowsLogicalTop.value
  )
})

function positiveModulo(value: number, divisor: number): number {
  return ((value % divisor) + divisor) % divisor
}

const placeholderNoneLogicalTop = computed(() => {
  const period = Math.max(placeholderNoneRowRefHeight.value + 2 * paddingPixel, 1)
  const relativeTop =
    positiveModulo(
      lastRowBottom.value - props.effectiveScrollTop + windowHeight.value,
      period
    ) - windowHeight.value
  return props.effectiveScrollTop + relativeTop
})

const placeholderNoneTopPixel = computed(() => {
  if (props.scrollMode === 'native-bottom') {
    return projectVirtualBottom(
      placeholderNoneLogicalTop.value,
      props.logicalUpperBound,
      windowHeight.value
    )
  }
  return projectVirtualTop(placeholderNoneLogicalTop.value, props.projectionOrigin)
})

useFetchImgs(visibleRows, visibleRowsLength, batchNumber, props.isolationId)
useFetchRows(startHeight, endHeight, props.isolationId)

watch(windowWidth, () => {
  visibleRows.value = []
})
</script>

<template>
  <div
    id="buffer"
    class="position-relative w-100 overflow-y-hidden"
    :style="{
      height: `${physicalBufferHeight}px`
    }"
  >
    <div
      v-if="visibleRows.length > 0"
      class="buffer-visible-rows position-absolute w-100"
      :style="visibleRowsProjectionStyle"
    >
      <BufferPlaceholder
        v-if="prefetchStore.totalHeight > windowHeight"
        id="placeholderTop"
        :top-pixel="0"
        :modify-top-pixel="true"
      />
      <div
        v-for="row in visibleRows"
        :key="`${row.start}-${prefetchStore.timestamp}`"
        v-memo="[
          row,
          row.topPixelAccumulated,
          row.offset,
          row.rowHeight,
          row.start,
          row.end,
          row.displayElements,
          visibleRowsLogicalTop,
          prefetchStore.timestamp
        ]"
        class="position-absolute w-100"
        :style="{
          transform: `translateY(${projectRelativeTop(
            row.topPixelAccumulated + row.offset,
            visibleRowsLogicalTop
          )}px)`,
          height: `${row.rowHeight}px`
        }"
        :start="row.start"
      >
        <RowBlock :row="row" :isolation-id="isolationId" />
      </div>
      <BufferPlaceholder
        v-if="prefetchStore.totalHeight > windowHeight"
        id="placeholderBottom"
        :top-pixel="placeholderBottomTop"
        :modify-top-pixel="false"
      />
    </div>
    <BufferPlaceholder
      v-if="rowStore.firstRowFetched && visibleRows.length === 0 && windowWidth > 0"
      id="placeholderNone"
      ref="placeholderNoneRef"
      :top-pixel="placeholderNoneTopPixel"
      :modify-top-pixel="false"
      :anchor-from-bottom="scrollMode === 'native-bottom'"
    />
  </div>
</template>
