<template>
  <v-sheet class="position-fixed w-100 h-100" :style="{ backgroundColor: `#424242` }">
    <v-sheet
      v-if="imageContainerRef"
      ref="scrollbarRef"
      class="position-fixed"
      id="scroll-bar"
      :style="{
        width: `${scrollBarWidth}px`,
        height: `calc(100% - 100px)`,
        right: `${0}`,
        zIndex: `3`,
        cursor: `vertical-text`,
        backgroundColor: `#424242`,
        marginTop: '8px'
      }"
      @click="handleClick"
      @mousedown="handleMouseDown"
      @mouseup="handleMouseUp"
      @mousemove="handleHover"
      @mouseleave="handleMouseLeave"
      @touchstart="handleTouchStart"
      @touchend="handleTouchEnd"
      @touchmove="handleMove"
    >
      <v-sheet
        class="position-relative w-100 h-100"
        :style="{
          pointerEvents: 'none'
        }"
      >
        <!-- Sheet to used as a visual indicator (a dash line) representing the timestamp of the currentscroll position within the view. -->
        <v-sheet
          v-if="scrollbarStore.initialized"
          id="current-date-chip"
          class="w-100 position-absolute"
          :style="{
            height: `calc(${(currentDateChipIndex / rowLength) * 100}% + 1px)`,
            borderBottom: '1px solid deepskyblue'
          }"
        ></v-sheet>
        <!-- Chips to show the all year labels. -->
        <v-chip
          v-for="scrollbarData in displayScrollbarDataArrayYear"
          :key="scrollbarData.index"
          size="x-small"
          variant="text"
          class="w-100 position-absolute text-grey-lighten-2 pa-0 ma-0 d-flex align-center justify-center"
          :style="{
            top: `${(Math.floor(scrollbarData.index / layoutBatchNumber) / rowLength) * 100}%`,
            userSelect: 'none'
          }"
        >
          {{ scrollbarData.year }}
        </v-chip>
        <!-- This sheet's height is adjusted to visually indicate the size of the current row block. -->
        <v-sheet
          v-if="scrollbarRef"
          id="block-chip"
          class="w-100 position-absolute bg-grey-darken-2"
          :style="{
            height: `${scrollbarHeight / rowLength}px`,
            top: `${(hoverLabelRowIndex / rowLength) * 100}%`
          }"
        >
          <!-- Chip to show the current view year and month label. -->
          <v-sheet
            id="day-chip"
            class="position-absolte w-100 d-flex align-center justify-center text-caption"
            :style="{
              backgroundColor: '#3a3a3a',
              borderTop: '1px solid deepskyblue',
              height: `25px`,
              zIndex: 2,
              userSelect: 'none'
            }"
          >
            {{ hoverLabelDate }}</v-sheet
          >
        </v-sheet>
      </v-sheet>
    </v-sheet>
  </v-sheet>
</template>

<script setup lang="ts">
import { ref, inject, Ref, computed, watch, watchEffect, onMounted, onBeforeUnmount } from 'vue'
import { clamp, debounce } from 'lodash'
import { useElementSize, useMouseInElement } from '@vueuse/core'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useRowStore } from '@/store/rowStore'
import { useOffsetStore } from '@/store/offsetStore'
import { useQueueStore } from '@/store/queueStore'
import { useLocationStore } from '@/store/locationStore'
import { fetchRowInWorker } from '@/script/inWorker/fetchRowInWorker'
import { IsolationId, ScrollbarData } from '@/script/common/types'
import { fixedBigRowHeight, layoutBatchNumber, scrollBarWidth } from '@/script/common/constants'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { getInjectValue, getScrollUpperBound } from '@utils/getter'
const isScrolling = ref(false)
const hoverLabelRowIndex = ref(0)
const currentDateChipIndex = ref(0)
const chipSize = 25

const props = defineProps<{
  isolationId: IsolationId
}>()

const scrollTopStore = useScrollTopStore(props.isolationId)
const locationStore = useLocationStore(props.isolationId)
const prefetchStore = usePrefetchStore(props.isolationId)
const scrollbarStore = useScrollbarStore(props.isolationId)
const rowStore = useRowStore(props.isolationId)
const offsetStore = useOffsetStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const windowHeight = getInjectValue<Ref<number>>('windowHeight')

const reachBottom = computed(() => {
  return (
    scrollTopStore.scrollTop ===
    Math.max(getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value), 0)
  )
})

const imageContainerRef = inject<Ref<HTMLElement | null>>('imageContainerRef')
const scrollbarRef = ref<HTMLElement | null>(null)

const rowLength = computed(() => prefetchStore.rowLength)
const { height: scrollbarHeight } = useElementSize(scrollbarRef)
const scrollbarMouse = useMouseInElement(scrollbarRef)

/**
 * Calculate the height of a single row chip.
 */
const singleRowChipHeight = computed(() => scrollbarHeight.value / rowLength.value)

/**
 * Compute the minimum number of row indices needed to separate batches.
 */
const rowIndexDifferenceLowerBound = computed(() => Math.ceil(chipSize / singleRowChipHeight.value))

/**
 * Index of the first batch that appears (partially) in the viewport.
 */
const currentBatchIndex = computed(() =>
  Math.floor(locationStore.locationIndex / layoutBatchNumber)
)

/**
 * Get the hover label's corresponding date based on the row index.
 */
const hoverLabelDate = computed(() => {
  let returnedString = ''
  for (let scrollbarData of scrollbarStore.scrollbarDataArray) {
    const scrollbarDataRowIndex = Math.floor(scrollbarData.index / layoutBatchNumber)
    if (hoverLabelRowIndex.value >= scrollbarDataRowIndex) {
      returnedString = `${scrollbarData.year}.${scrollbarData.month}`
    } else {
      break
    }
  }
  return returnedString
})

const displayScrollbarDataArrayYear: Ref<ScrollbarData[]> = ref([])

const getTargetRowIndex = (percentage: number) => {
  /**
   * Given a percentage t of scrollbar height, return the corresponding row index k, where n = rowLength - 1.
   *
   * 0───┐<─── 0% height
   *     │
   * 1───┤
   *     │
   * 2───┤
   *     │
   *     ⋮
   * k───┤
   *     │<─── t% height
   * k+1─┤
   *     │
   *     ⋮
   * n───┤
   *     │
   * ────┘<─── 100% height
   */
  const targetRowIndex = Math.floor(rowLength.value * percentage)
  return clamp(targetRowIndex, 0, rowLength.value - 1)
}

const debouncedFetchRow = debounce((index: number) => {
  fetchRowInWorker(index, props.isolationId)
}, 100)

/**
 * Handle a click event on the scrollbar.
 */
const handleClick = () => {
  const clickPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
  const targetRowIndex = getTargetRowIndex(clickPositionRelative / scrollbarHeight.value)

  if (targetRowIndex === currentDateChipIndex.value) {
    return
  }

  currentDateChipIndex.value = targetRowIndex
  locationStore.anchor = targetRowIndex
  offsetStore.clearAll()
  queueStore.clearAll()
  prefetchStore.clearForResize()
  rowStore.clearForResize()
  scrollTopStore.scrollTop = targetRowIndex * fixedBigRowHeight
  debouncedFetchRow(targetRowIndex)
}

/**
 * Handle movement over the scrollbar.
 */
const handleMove = () => {
  if (scrollbarStore.isDragging) {
    const hoverPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
    const targetRowIndex = getTargetRowIndex(hoverPositionRelative / scrollbarHeight.value)

    if (targetRowIndex >= 0 && targetRowIndex <= rowLength.value - 1) {
      handleClick()
      hoverLabelRowIndex.value = targetRowIndex
    }
  }
}

const handleHover = () => {
  const hoverPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
  const targetRowIndex = getTargetRowIndex(hoverPositionRelative / scrollbarHeight.value)

  if (targetRowIndex >= 0 && targetRowIndex <= rowLength.value - 1) {
    hoverLabelRowIndex.value = targetRowIndex
  }
}

const handleMouseDown = () => {
  isScrolling.value = true
  scrollbarStore.isDragging = true
}

const handleMouseUp = () => {
  scrollbarStore.isDragging = false
}

const handleMouseLeave = () => {
  if (reachBottom.value) {
    hoverLabelRowIndex.value = rowLength.value - 1
  } else {
    hoverLabelRowIndex.value = currentBatchIndex.value
  }
}

const handleTouchStart = () => {
  isScrolling.value = true
  scrollbarStore.isDragging = true
  handleClick()
}

const handleTouchEnd = () => {
  scrollbarStore.isDragging = false
}

/**
 * Watch for changes in scrollbar data and update the displayed year data array.
 */
watchEffect(() => {
  const array: ScrollbarData[] = []
  let lastIndex: number | null = null

  scrollbarStore.scrollbarDataArrayYear.forEach((scrollbarData) => {
    const index = Math.floor(scrollbarData.index / layoutBatchNumber)
    if (
      lastIndex === null ||
      (index - lastIndex >= rowIndexDifferenceLowerBound.value &&
        index < rowLength.value - rowIndexDifferenceLowerBound.value)
    ) {
      lastIndex = index
      array.push(scrollbarData)
    }
  })
  displayScrollbarDataArrayYear.value = array
})

/**
 * Watch for changes in location index and update scroll state accordingly.
 */

watch([() => locationStore.locationIndex, reachBottom], () => {
  isScrolling.value = true
  if (reachBottom.value) {
    hoverLabelRowIndex.value = rowLength.value - 1
    currentDateChipIndex.value = rowLength.value - 1
  } else {
    hoverLabelRowIndex.value = currentBatchIndex.value
    currentDateChipIndex.value = currentBatchIndex.value
  }
})

onMounted(() => {
  window.addEventListener('mouseup', handleMouseUp)
  window.addEventListener('mousemove', handleMove)
})

onBeforeUnmount(() => {
  window.removeEventListener('mouseup', handleMouseUp)
  window.removeEventListener('mousemove', handleMove)
})
</script>
