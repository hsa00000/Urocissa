<template>
  <div class="h-100 py-2" :style="{ width: `${scrollBarWidth}px` }">
    <div
      v-if="imageContainerRef"
      class="h-100 v-sheet"
      ref="scrollbarRef"
      id="scroll-bar"
      :style="{
        zIndex: `3`,
        cursor: `vertical-text`,
        touchAction: 'none',
        overscrollBehavior: 'contain'
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
        id="main-sheet"
        class="position-relative w-100 h-100"
        :style="{
          pointerEvents: 'none'
        }"
      >
        <v-sheet
          v-if="scrollbarRef"
          class="w-100 position-absolute bg-transparent"
          :style="{
            height: `${scrollbarHeight / rowLength}px`,
            top: `${(currentDateChipIndex / rowLength) * 100}%`,
            borderBottom: '1px solid rgb(var(--v-theme-primary))'
          }"
        >
        </v-sheet>
        <!-- Chips to show the all year labels. -->
        <v-chip
          v-for="scrollbarData in displayScrollbarDataArrayYear"
          :key="scrollbarData.index"
          size="x-small"
          variant="text"
          class="w-100 position-absolute pa-0 ma-0 d-flex align-center justify-center"
          :style="{
            top: `${(Math.floor(scrollbarData.index / layoutBatchNumber) / rowLength) * 100}%`,
            userSelect: 'none',
            zIndex: 3
          }"
        >
          {{ scrollbarData.year }}
        </v-chip>
        <!-- This sheet's height is adjusted to visually indicate the size of the current row block. -->
        <v-sheet
          v-if="scrollbarRef && hoverLabelRowIndex !== undefined"
          id="current-block-sheet"
          :class="[
            'w-100 position-absolute',
            scrollbarStore.isHovering || scrollbarStore.isDragging
              ? 'bg-surface-light'
              : 'bg-surface'
          ]"
          :style="{
            height: `${scrollbarHeight / rowLength}px`,
            top: `${(hoverLabelRowIndex / rowLength) * 100}%`,
            borderBottom: '1px solid rgb(var(--v-theme-primary))'
          }"
        >
          <!-- Chip to show the current view year and month label. Only render while mouse is over the scrollbar. -->

          <v-sheet
            v-if="
              hoverLabelDate !== undefined &&
              scrollbarRef &&
              (configStore.isMobile
                ? scrollbarStore.isDragging
                : scrollbarStore.isHovering || scrollbarStore.isDragging)
            "
            id="current-month-sheet"
            class="position-absolute w-100 d-flex align-center justify-center text-caption bg-surface"
            :style="{
              height: `25px`,
              bottom: `0`,
              left: `-${scrollBarWidth + 8}px`,
              zIndex: 4,
              userSelect: 'none'
            }"
          >
            {{ hoverLabelDate }}
          </v-sheet>
        </v-sheet>
      </v-sheet>
    </div>
  </div>
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
import { fetchRowInWorker } from '@/api/fetchRow'
import { IsolationId, ScrollbarData } from '@type/types'
import { fixedBigRowHeight, layoutBatchNumber, scrollBarWidth } from '@/type/constants'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { getInjectValue, getScrollUpperBound } from '@utils/getter'
import { useConfigStore } from '@/store/configStore'
const isScrolling = ref(false)
const hoverLabelRowIndex: Ref<number | undefined> = ref(undefined)
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
const configStore = useConfigStore('mainId')

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
  const h = hoverLabelRowIndex.value
  if (h === undefined) return undefined
  let label: string | undefined
  for (const d of scrollbarStore.scrollbarDataArray) {
    const rowIdx = Math.floor(d.index / layoutBatchNumber)
    if (h >= rowIdx) {
      label = `${d.year}.${d.month}`
    } else {
      break
    }
  }
  return label
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
  fetchRowInWorker(index, props.isolationId).catch((err: unknown) => {
    console.error('fetchRowInWorker failed:', err)
  })
}, 100)

/**
 * Get relative Y position from event and scrollbar element
 */
const getRelativePosition = (event: MouseEvent | TouchEvent): number => {
  const element = scrollbarRef.value
  if (!element) return 0

  const rect = element.getBoundingClientRect()
  let clientY: number

  if ('touches' in event && event.touches.length > 0) {
    // Touch event
    const touch = event.touches[0]
    if (!touch) return 0
    clientY = touch.clientY
  } else if ('clientY' in event) {
    // Mouse event
    clientY = event.clientY
  } else {
    return 0
  }

  const relativeY = clientY - rect.top
  return Math.max(0, Math.min(relativeY, rect.height))
}

/**
 * Handle a click event on the scrollbar.
 */
const handleClick = (event?: MouseEvent | TouchEvent) => {
  let clickPositionRelative: number

  if (event) {
    // Use event position for immediate response
    clickPositionRelative = getRelativePosition(event)
  } else {
    // Fallback to mouse tracking (for legacy click handler)
    clickPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
  }

  const targetRowIndex = getTargetRowIndex(clickPositionRelative / scrollbarHeight.value)

  if (targetRowIndex === currentDateChipIndex.value) {
    return
  }

  locationStore.anchor = targetRowIndex

  offsetStore.clearAll()
  queueStore.clearAll()
  prefetchStore.clearForResize()
  rowStore.clearForResize()
  scrollTopStore.scrollTop = targetRowIndex * fixedBigRowHeight
  currentDateChipIndex.value = targetRowIndex
  hoverLabelRowIndex.value = targetRowIndex
  debouncedFetchRow(targetRowIndex)
}

/**
 * Handle movement over the scrollbar.
 */
const handleMove = (event?: MouseEvent | TouchEvent) => {
  if (scrollbarStore.isDragging) {
    let hoverPositionRelative: number

    if (event) {
      // Use event position for immediate response
      hoverPositionRelative = getRelativePosition(event)
    } else {
      // Fallback to mouse tracking
      hoverPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
    }

    const targetRowIndex = getTargetRowIndex(hoverPositionRelative / scrollbarHeight.value)

    if (targetRowIndex >= 0 && targetRowIndex <= rowLength.value - 1) {
      handleClick(event)
    }
  }
}

const handleHover = (event?: MouseEvent) => {
  let hoverPositionRelative: number

  if (event) {
    // Use event position for immediate response
    hoverPositionRelative = getRelativePosition(event)
  } else {
    // Fallback to mouse tracking
    hoverPositionRelative = Math.max(0, scrollbarMouse.elementY.value)
  }

  const targetRowIndex = getTargetRowIndex(hoverPositionRelative / scrollbarHeight.value)

  if (targetRowIndex >= 0 && targetRowIndex <= rowLength.value - 1) {
    hoverLabelRowIndex.value = targetRowIndex
  }
  scrollbarStore.isHovering = true
}

const handleMouseDown = (event: MouseEvent) => {
  isScrolling.value = true
  scrollbarStore.isDragging = true
  handleClick(event)
}

const handleMouseUp = () => {
  scrollbarStore.isDragging = false
}

const handleMouseLeave = () => {
  // Hide hover label when cursor leaves the scrollbar

  hoverLabelRowIndex.value = undefined
  scrollbarStore.isHovering = false
}

const handleTouchStart = (event: TouchEvent) => {
  isScrolling.value = true
  scrollbarStore.isDragging = true
  handleClick(event)
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
  hoverLabelRowIndex.value = undefined
  if (reachBottom.value) {
    currentDateChipIndex.value = rowLength.value - 1
  } else {
    currentDateChipIndex.value = currentBatchIndex.value
  }
})

onMounted(() => {
  const handleGlobalMouseMove = (event: MouseEvent) => {
    handleMove(event)
  }

  window.addEventListener('mouseup', handleMouseUp)
  window.addEventListener('mousemove', handleGlobalMouseMove)

  onBeforeUnmount(() => {
    window.removeEventListener('mouseup', handleMouseUp)
    window.removeEventListener('mousemove', handleGlobalMouseMove)
  })
})
</script>
