<template>
  <div class="w-100 d-flex flex-wrap no-select">
    <div
      v-for="(data, subIndex) in row.displayElements"
      :key="`${row.start}-${subIndex}-${prefetchStore.timestamp}`"
      data-testid="gallery-item"
      :data-item-index="row.start + subIndex"
      :style="{
        width: `${data.displayWidth}px`,
        height: `${data.displayHeight}px`
      }"
      class="ma-1"
    >
      <div class="position-relative w-100 h-100 parent">
        <div
          id="click-handler"
          :class="['w-100 h-100 position-absolute', { 'locate-highlight': locationStore.highlightedIndex === row.start + subIndex }]"
          :style="{
            pointerEvents: 'none',
            zIndex: 100,
            border:
              collectionStore.editModeOn &&
              collectionStore.isSelected(row.start + subIndex)
                ? '4px solid rgb(var(--v-theme-primary))'
                : '4px solid transparent'
          }"
          @click="(event: MouseEvent) => handleClick(event, row.start + subIndex)"
        ></div>
        <DesktopHoverIcon
          class="icon-hover child"
          v-if="!mobile"
          :on-click="(event: MouseEvent) => handleClickIcon(event, row.start + subIndex)"
        />
        <HoverGradientDiv :mobile="mobile" />
        <MainBlock
          v-if="subIndex < timeInterval"
          :index="row.start + subIndex"
          :display-element="data"
          :isolation-id="props.isolationId"
          :mobile="mobile"
          :on-pointerdown="(event: PointerEvent) => handlePointerdown(event, row.start + subIndex)"
          :on-pointerup="(event: PointerEvent) => handlePointerUp(event, row.start + subIndex)"
          :on-pointerleave="handlePointerLeave"
          :on-click="(event: MouseEvent) => handleClick(event, row.start + subIndex)"
        />
        <div
          id="grey-background-placeholder"
          data-testid="open-item"
          role="button"
          aria-label="Open item"
          class="w-100 h-100 bg-placeholder position-absolute"
          :style="{
            zIndex: 0
          }"
          @click="(event: MouseEvent) => handleClick(event, row.start + subIndex)"
        ></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { layoutBatchNumber } from '@/type/constants'
import { IsolationId, Row } from '@type/types'
import { useCollectionStore } from '@/store/collectionStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useHandleClick } from '@/script/hook/useHandleClick'
import { useRouter, useRoute } from 'vue-router'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { getArrayValue } from '@utils/getter'
import { useScrollTopStore } from '@/store/scrollTopStore'
import MainBlock from './FunctionalComponent/MainBlock'
import DesktopHoverIcon from './FunctionalComponent/DesktopHoverIcon'
import HoverGradientDiv from './FunctionalComponent/HoverGradientDiv'
import { useConfigStore } from '@/store/configStore'
import { useConstStore } from '@/store/constStore'
import { useLocationStore } from '@/store/locationStore'
const props = defineProps<{
  row: Row
  isolationId: IsolationId
}>()

const router = useRouter()
const route = useRoute()
const constStore = useConstStore('mainId')
const configStore = useConfigStore('mainId')
const prefetchStore = usePrefetchStore(props.isolationId)
const collectionStore = useCollectionStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const workerStore = useWorkerStore(props.isolationId)
const scorllTopStore = useScrollTopStore(props.isolationId)
const locationStore = useLocationStore(props.isolationId)
const timeInterval = ref(0)
const isLongPress = ref(false)
const pressTimer = ref<number | null>(null)
const scrollingTimer = ref<number | null>(null)
const isScrolling = ref(false)

const mobile = configStore.isMobile

watch(
  () => scorllTopStore.scrollTop,
  () => {
    isScrolling.value = true

    if (scrollingTimer.value !== null) {
      clearTimeout(scrollingTimer.value)
    }

    scrollingTimer.value = window.setTimeout(() => {
      isScrolling.value = false

      scrollingTimer.value = null
    }, 100)
  }
)

const { handleClick } = useHandleClick(router, route, props.isolationId)

const handlePointerdown = (event: MouseEvent, currentIndex: number) => {
  if (isScrolling.value) {
    return
  }
  isLongPress.value = false
  pressTimer.value = window.setTimeout(() => {
    isLongPress.value = true
    handleLongPressClick(event, currentIndex)
  }, 600)
}

const handlePointerUp = (event: MouseEvent, currentIndex: number) => {
  if (isScrolling.value) {
    return
  }
  if (pressTimer.value !== null) {
    clearTimeout(pressTimer.value)
    pressTimer.value = null
  }
  if (!isLongPress.value) {
    handleClick(event, currentIndex)
  }
}

const handlePointerLeave = () => {
  if (pressTimer.value !== null) {
    clearTimeout(pressTimer.value)
    pressTimer.value = null
  }
}

const handleLongPressClick = (event: MouseEvent, currentIndex: number) => {
  if (!collectionStore.editModeOn) {
    collectionStore.editModeOn = true
    collectionStore.addApi(currentIndex)
    collectionStore.lastClick = currentIndex
  } else {
    handleClick(event, currentIndex)
  }
}
const handleClickIcon = (event: MouseEvent, currentIndex: number) => {
  if (!collectionStore.editModeOn) {
    collectionStore.editModeOn = true
    collectionStore.addApi(currentIndex)
    collectionStore.lastClick = currentIndex
  } else {
    handleClick(event, currentIndex)
  }
}

watch(
  () => locationStore.highlightedIndex,
  (val) => {
    if (val !== null && val >= props.row.start && val <= props.row.end) {
      setTimeout(() => {
        locationStore.highlightedIndex = null
      }, 2000)
    }
  }
)

onMounted(() => {
  const intervalId = setInterval(() => {
    // this part is crutial: if we do not delay the show of img, the scrub will lag if the img already loading
    if (timeInterval.value < layoutBatchNumber) {
      timeInterval.value += layoutBatchNumber
    } else {
      clearInterval(intervalId)
    }
  }, 0)
})

onBeforeUnmount(() => {
  for (let abortIndex = props.row.start; abortIndex <= props.row.end; abortIndex++) {
    const workerIndex = abortIndex % constStore.concurrencyNumber
    if (workerStore.postToImgWorkerList !== undefined) {
      getArrayValue(workerStore.postToImgWorkerList, workerIndex).processAbort({
        index: abortIndex
      })
    } else {
      console.error('workerStore.postToImgWorkerList is undefined')
    }
    queueStore.img.delete(abortIndex)
  }
})
</script>
<style scoped>
.parent:not(:hover) .child {
  display: none;
}
.icon-hover {
  color: #fafafa;
  transition: color 0.3s;
  cursor: pointer;
}

.icon-hover:hover {
  color: white;
}

.locate-highlight {
  animation: locate-pulse 2s ease-out forwards;
}

@keyframes locate-pulse {
  0% {
    box-shadow: inset 0 0 0 4px rgba(255, 193, 7, 0.9);
  }
  100% {
    box-shadow: inset 0 0 0 4px transparent;
  }
}
</style>
