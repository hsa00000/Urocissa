<template>
  <div
    id="buffer"
    class="position-relative w-100 overflow-y-hidden"
    :style="{
      height: `${Math.max(bufferHeight, prefetchStore.totalHeight)}px`
    }"
  >
    <div
      v-if="visibleRows.length > 0"
      class="buffer-visible-rows position-absolute w-100"
      :style="{
        transform: `translateY(${projectVirtualTop(
          visibleRowsLogicalTop,
          scrollTopStore.scrollTop,
          bufferHeight
        )}px)`,
        willChange: 'transform'
      }"
    >
      <BufferPlaceholder
        id="placeholderTop"
        v-if="!(prefetchStore.totalHeight <= windowHeight)"
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
            row.topPixelAccumulated! + row.offset,
            visibleRowsLogicalTop
          )}px)`,
          height: `${row.rowHeight}px`
        }"
        :start="row.start"
      >
        <RowBlock :row="row" :isolation-id="isolationId" />
      </div>
      <BufferPlaceholder
        id="placeholderBottom"
        v-if="!(prefetchStore.totalHeight <= windowHeight)"
        :top-pixel="(()=>{
          const lastData = getArrayValue(visibleRows, visibleRows.length - 1)
          return projectRelativeTop(
            lastData.topPixelAccumulated! + lastData.offset + lastData.rowHeight,
            visibleRowsLogicalTop
          )
        })()"
        :modify-top-pixel="false"
      />
    </div>
    <BufferPlaceholder
      id="placeholderNone"
      ref="placeholderNoneRef"
      v-if="rowStore.firstRowFetched && visibleRows.length === 0 && windowWidth > 0"
      :top-pixel="
        ((lastRowBottom - effectiveScrollTop + windowHeight) %
          (placeholderNoneRowRefHeight + 2 * paddingPixel)) +
        bufferHeight / 3 -
        windowHeight
      "
      :modify-top-pixel="false"
    />
  </div>
</template>

<script setup lang="ts">
/**
 * Before understanding this component, one should first understand how its parent component (image-container) works.
 * Refer to the comments in Home.vue.
 *
 * Buffer has a large height to ensure that the parent Homepage can scroll without reaching the top or bottom prematurely.
 *
 * Buffer component contains a list of RowBlocks, with BufferPlaceholders at the top (placeholderTop) and bottom (placeholderBottom) of this list.
 * The BufferPlaceholder is crucial for improving the perceived load time and smoothness of scrolling.
 * If the list of RowBlocks is empty, BufferPlaceholder (placeholderNone) will be displayed instead.
 *
 * `topPixelAccumulated` represents the top pixel position of a RowBlock.
 * Committed `scrollTop` controls the Buffer transform. While Chrome is performing native
 * scrolling, `effectiveScrollTop` additionally includes the physical buffer displacement so
 * rows can be selected and prefetched without interrupting the browser animation.
 * `bufferHeight / 3` is used to position the RowBlock at a sufficient distance from the top of the component so that the parent Homepage can scroll up without reaching the top prematurely.
 */
import { ComponentPublicInstance, Ref, computed, provide, ref, watch } from 'vue'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useFetchImgs } from '@/script/hook/useFetchImgs'
import { useUpdateVisibleRows } from '@/script/hook/useUpdateVisibleRows'
import { useFetchRows } from '@/script/hook/useFetchRows'
import { batchNumber, paddingPixel } from '@/type/constants'
import BufferPlaceholder from '@/components/Buffer/BufferPlaceholder.vue'
import RowBlock from '@/components/Buffer/BufferRowBlock.vue'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { getArrayValue, getInjectValue } from '@utils/getter'
import { IsolationId } from '@type/types'
import { useRowStore } from '@/store/rowStore'
import { scrollActivityKey, useScrollActivity } from '@/script/hook/useScrollActivity'
import { projectRelativeTop, projectVirtualTop } from '@/script/utils/rowOffset'

const props = defineProps<{
  isolationId: IsolationId
  bufferHeight: number
  effectiveScrollTop: number
}>()

const prefetchStore = usePrefetchStore(props.isolationId)
const scrollTopStore = useScrollTopStore(props.isolationId)
const rowStore = useRowStore(props.isolationId)
const scrollActivity = useScrollActivity(() => props.effectiveScrollTop)
provide(scrollActivityKey, scrollActivity)

const windowWidth = getInjectValue<Ref<number>>('windowWidth')
const windowHeight = getInjectValue<Ref<number>>('windowHeight')
const imageContainerRef = getInjectValue<Ref<HTMLElement>>('imageContainerRef')

type BufferPlaceholderInstance = ComponentPublicInstance<{
  placeholderRowRefHeight: number
}>
const placeholderNoneRef = ref<BufferPlaceholderInstance | null>(null)
const lastRowBottom = ref(0)

const placeholderNoneRowRefHeight = computed(() =>
  placeholderNoneRef.value ? placeholderNoneRef.value.placeholderRowRefHeight : 0
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
  windowHeight,
  props.isolationId
)
useFetchImgs(visibleRows, visibleRowsLength, batchNumber, props.isolationId)
useFetchRows(startHeight, endHeight, props.isolationId)

watch(windowWidth, () => {
  visibleRows.value = []
})
</script>
