<template>
  <div class="w-100 d-flex flex-wrap row-flex-container no-select">
    <div
      v-for="(data, subIndex) in row.displayElements"
      :key="`${row.start}-${subIndex}-${prefetchStore.timestamp}`"
      :style="{
        width: `${data.displayWidth}px`,
        height: `${data.displayHeight}px`
      }"
      class="ma-1"
    >
      <div
        class="position-relative w-100 h-100 parent"
        :style="{
          border:
            collectionStore.editModeOn &&
            collectionStore.editModeCollection.has(row.start + subIndex)
              ? '4px solid #81D4FA'
              : ''
        }"
      >
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
          :style="{
            position: 'absolute',
            zIndex: 0
          }"
          @click="(event: MouseEvent) => handleClick(event, row.start + subIndex)"
          class="w-100 h-100 bg-grey-darken-2"
        ></div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { layoutBatchNumber } from '@/script/common/constants'
import { IsolationId, Row } from '@/script/common/types'
import { useCollectionStore } from '@/store/collectionStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { onBeforeUnmount, onMounted, ref, watch } from 'vue'
import { useHandleClick } from '@/components/hook/useHandleClick'
import { useRouter, useRoute } from 'vue-router'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { getArrayValue, getInjectValue } from '@utils/getter'
import { useScrollTopStore } from '@/store/scrollTopStore'
import MainBlock from './FunctionalComponent/MainBlock'
import DesktopHoverIcon from './FunctionalComponent/DesktopHoverIcon'
import HoverGradientDiv from './FunctionalComponent/HoverGradientDiv'
const props = defineProps<{
  row: Row
  isolationId: IsolationId
}>()

const router = useRouter()
const route = useRoute()
const mobile = getInjectValue<boolean>('mobile')
const prefetchStore = usePrefetchStore(props.isolationId)
const collectionStore = useCollectionStore(props.isolationId)
const queueStore = useQueueStore(props.isolationId)
const workerStore = useWorkerStore(props.isolationId)
const scorllTopStore = useScrollTopStore(props.isolationId)
const timeInterval = ref(0)

const isLongPress = ref(false)
const pressTimer = ref<number | null>(null) // 定時器 ID
const scrollingTimer = ref<number | null>(null)
const isScrolling = ref(false)

// Prevent accidental touches while scrolling
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
  isLongPress.value = false // 初始為非長按
  pressTimer.value = window.setTimeout(() => {
    isLongPress.value = true // 設置為長按
    handleLongPressClick(event, currentIndex) // 觸發長按事件
  }, 600) // 長按持續時間 (例如 800 毫秒)
}

const handlePointerUp = (event: MouseEvent, currentIndex: number) => {
  if (isScrolling.value) {
    return
  }
  if (pressTimer.value !== null) {
    clearTimeout(pressTimer.value) // 清除定時器
    pressTimer.value = null
  }
  if (!isLongPress.value) {
    handleClick(event, currentIndex) // 若非長按則觸發點擊事件
  }
}

const handlePointerLeave = () => {
  if (pressTimer.value !== null) {
    clearTimeout(pressTimer.value) // 取消長按事件
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
    const workerIndex = abortIndex % workerStore.concurrencyNumber
    if (workerStore.postToWorkerList !== undefined) {
      getArrayValue(workerStore.postToWorkerList, workerIndex).processAbort({
        index: abortIndex
      })
    } else {
      console.error('workerStore.postToWorkerList is undefined')
    }
    queueStore.img.delete(abortIndex)
  }
})
</script>
<style scoped>
.no-select {
  user-select: none;
}
.no-select * {
  user-select: none;
}
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
</style>
