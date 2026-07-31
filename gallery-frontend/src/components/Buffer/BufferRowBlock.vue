<script setup lang="ts">
import { computed, onBeforeUnmount, shallowRef, toRef, watch, type ObjectDirective } from 'vue'
import { basename, extname } from 'upath'
import type { IsolationId, Row } from '@type/types'
import { useCollectionStore } from '@/store/collectionStore'
import { useConfigStore } from '@/store/configStore'
import { useConstStore } from '@/store/constStore'
import { useLocationStore } from '@/store/locationStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useQueueStore } from '@/store/queueStore'
import { useWorkerStore } from '@/store/workerStore'
import { useHandleClick } from '@/script/hook/useHandleClick'
import { useRowMediaRequests } from '@/script/hook/useRowMediaRequests'
import { useRowScrollActivity } from '@/script/hook/useScrollActivity'
import { paddingPixel } from '@/type/constants'
import { formatDuration } from '@utils/dater'
import { getArrayValue } from '@utils/getter'
import {
  registerThumbnailElement,
  transparentThumbnailSrc,
  unregisterThumbnailElement
} from '@/script/utils/thumbnailElementRegistry'
import { useRoute, useRouter } from 'vue-router'
import AlbumChip from './FunctionalComponent/AlbumChip'
import DesktopHoverIcon from './FunctionalComponent/DesktopHoverIcon'
import DurationChip from './FunctionalComponent/DurationChip'
import FilenameChip from './FunctionalComponent/FilenameChip'
import ProcessingChip from './FunctionalComponent/ProcessingChip'

interface RenderTile {
  key: string
  index: number
  pending: boolean
  durationLabel: string | null
  filenameLabel: string | null
  albumLabel: string | null
  chipMaxWidth: string
  tileClass: string
  tileStyle: string
  thumbnailClass: string
  leftPixel: number
  topPixel: number
}

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
const locationStore = useLocationStore(props.isolationId)
const isScrolling = useRowScrollActivity()
const row = toRef(props, 'row')
const imagesDisabled = computed(() => configStore.disableImg)
const { dataStore, imgStore } = useRowMediaRequests(row, props.isolationId, imagesDisabled)
const vThumbnailSrc: ObjectDirective<HTMLImageElement, number> = {
  mounted(element, binding) {
    registerThumbnailElement(
      props.isolationId,
      binding.value,
      element,
      imgStore.imgUrl.get(binding.value)
    )
  },
  updated(element, binding) {
    if (binding.oldValue === binding.value) {
      return
    }

    if (binding.oldValue !== null) {
      unregisterThumbnailElement(props.isolationId, binding.oldValue, element)
    }
    registerThumbnailElement(
      props.isolationId,
      binding.value,
      element,
      imgStore.imgUrl.get(binding.value)
    )
  },
  unmounted(element, binding) {
    unregisterThumbnailElement(props.isolationId, binding.value, element)
  }
}
const hoveredIndex = shallowRef<number | null>(null)
const highlightTimers = new Set<ReturnType<typeof setTimeout>>()
let isLongPress = false
let pressTimer: number | null = null

// Device interaction behavior intentionally follows the existing physical-device flag,
// not a responsive viewport breakpoint.
const mobile = configStore.isMobile
const { handleClick } = useHandleClick(router, route, props.isolationId)

const tiles = computed<RenderTile[]>(() => {
  const timestamp = prefetchStore.timestamp
  const disableImg = imagesDisabled.value
  const showFilenameChip = constStore.showFilenameChip
  const editModeOn = collectionStore.editModeOn
  const highlightedIndex = locationStore.highlightedIndex
  let currentTopPixel = Number.NaN
  let currentLeftPixel = paddingPixel

  return props.row.displayElements.map((displayElement, subIndex) => {
    if (displayElement.displayTopPixelAccumulated !== currentTopPixel) {
      currentTopPixel = displayElement.displayTopPixelAccumulated
      currentLeftPixel = paddingPixel
    }

    const index = props.row.start + subIndex
    const abstractData = dataStore.data.get(index)
    const isMedia = abstractData?.type === 'image' || abstractData?.type === 'video'
    const duration = isMedia ? abstractData.exif.duration : undefined
    const file = isMedia ? abstractData.alias[0]?.file : undefined
    const base = file === undefined ? null : basename(file)
    const extension = base === null ? '' : extname(base)
    const filenameLabel =
      base === null || extension.length === 0 ? base : base.slice(0, -extension.length)
    const hasBorder = abstractData?.type === 'album'
    const selected = editModeOn && collectionStore.isSelected(index)
    const highlighted = highlightedIndex === index
    const thumbhashUrl =
      disableImg || typeof abstractData?.thumbhashUrl !== 'string'
        ? null
        : abstractData.thumbhashUrl
    let tileClass = 'gallery-tile position-relative ma-1 bg-placeholder'
    if (mobile) tileClass += ' gallery-tile--mobile'
    if (selected) tileClass += ' gallery-tile--selected'
    if (highlighted) tileClass += ' locate-highlight'
    const tileStyle =
      `width:${displayElement.displayWidth}px;height:${displayElement.displayHeight}px;` +
      (thumbhashUrl === null ? '' : `background-image:url('${thumbhashUrl}');`)
    const thumbnailClass =
      `thumbnail-image ${mobile ? 'mobile-small-image' : 'desktop-small-image'} ` +
      `w-100 h-100 position-absolute${hasBorder ? ' thumbnail-image--album' : ''}`
    const tile = {
      key: `${props.row.start}-${subIndex}-${timestamp}`,
      index,
      pending: isMedia && abstractData.pending,
      durationLabel: duration === undefined ? null : formatDuration(duration),
      filenameLabel: showFilenameChip ? filenameLabel : null,
      albumLabel: abstractData?.type === 'album' ? (abstractData.title ?? 'Untitled') : null,
      chipMaxWidth: `${(displayElement.displayWidth - 16) * 0.75}px`,
      tileClass,
      tileStyle,
      thumbnailClass,
      leftPixel: currentLeftPixel,
      topPixel: displayElement.displayTopPixelAccumulated + paddingPixel
    }
    currentLeftPixel += displayElement.displayWidth + 2 * paddingPixel
    return tile
  })
})
const hoveredTile = computed(() => {
  const index = hoveredIndex.value
  return index === null ? null : (tiles.value.find((tile) => tile.index === index) ?? null)
})

function getCurrentTargetIndex(event: Event): number {
  const target = event.currentTarget
  if (!(target instanceof HTMLElement)) {
    throw new Error('gallery interaction target is not an HTMLElement')
  }

  const index = Number(target.dataset.itemIndex)
  if (!Number.isInteger(index)) {
    throw new Error('gallery interaction target is missing data-item-index')
  }
  return index
}

function getEventTile(event: Event): HTMLElement | null {
  const target = event.target
  const currentTarget = event.currentTarget
  if (!(target instanceof Element) || !(currentTarget instanceof HTMLElement)) {
    return null
  }

  const tile = target.closest<HTMLElement>('.gallery-tile[data-item-index]')
  return tile !== null && currentTarget.contains(tile) ? tile : null
}

function getEventTileIndex(event: Event): number | null {
  const tile = getEventTile(event)
  if (tile === null) {
    return null
  }

  const index = Number(tile.dataset.itemIndex)
  return Number.isInteger(index) ? index : null
}

function handleOpenClick(event: MouseEvent): void {
  const index = getEventTileIndex(event)
  if (index !== null) {
    handleClick(event, index)
  }
}

function handlePointerdown(event: PointerEvent): void {
  if (isScrolling.value) {
    return
  }

  const currentIndex = getEventTileIndex(event)
  if (currentIndex === null) {
    return
  }

  isLongPress = false
  pressTimer = window.setTimeout(() => {
    isLongPress = true
    handleLongPressClick(event, currentIndex)
  }, 600)
}

function handlePointerUp(event: PointerEvent): void {
  if (isScrolling.value) {
    return
  }

  const currentIndex = getEventTileIndex(event)
  if (currentIndex === null) {
    return
  }

  if (pressTimer !== null) {
    clearTimeout(pressTimer)
    pressTimer = null
  }
  if (!isLongPress) {
    handleClick(event, currentIndex)
  }
}

function handlePointerOut(event: PointerEvent): void {
  const tile = getEventTile(event)
  if (tile === null) {
    return
  }

  const relatedTarget = event.relatedTarget
  if (relatedTarget instanceof Node && tile.contains(relatedTarget)) {
    return
  }

  if (pressTimer !== null) {
    clearTimeout(pressTimer)
    pressTimer = null
  }
}

function handleContextMenu(event: Event): void {
  if (getEventTile(event) !== null) {
    event.preventDefault()
  }
}

function handleLongPressClick(event: MouseEvent, currentIndex: number): void {
  if (!collectionStore.editModeOn) {
    collectionStore.editModeOn = true
    collectionStore.addApi(currentIndex)
    collectionStore.lastClick = currentIndex
  } else {
    handleClick(event, currentIndex)
  }
}

function handleSelectClick(event: MouseEvent): void {
  const currentIndex = getCurrentTargetIndex(event)
  if (!collectionStore.editModeOn) {
    collectionStore.editModeOn = true
    collectionStore.addApi(currentIndex)
    collectionStore.lastClick = currentIndex
  } else {
    handleClick(event, currentIndex)
  }
}

function handleMouseOver(event: MouseEvent): void {
  if (mobile) {
    return
  }

  const tile = getEventTile(event)
  if (tile === null) {
    return
  }

  const relatedTarget = event.relatedTarget
  if (relatedTarget instanceof Node && tile.contains(relatedTarget)) {
    return
  }

  const index = Number(tile.dataset.itemIndex)
  if (Number.isInteger(index)) {
    hoveredIndex.value = index
  }
}

function handleMouseOut(event: MouseEvent): void {
  const tile = getEventTile(event)
  if (tile === null) {
    const target = event.target
    if (
      target instanceof Element &&
      target.closest('[data-testid="select-item"]') !== null
    ) {
      hoveredIndex.value = null
    }
    return
  }

  const relatedTarget = event.relatedTarget
  if (relatedTarget instanceof Node && tile.contains(relatedTarget)) {
    return
  }

  const index = Number(tile.dataset.itemIndex)
  if (
    relatedTarget instanceof Element &&
    Number(relatedTarget.closest<HTMLElement>('[data-testid="select-item"]')?.dataset.itemIndex) ===
      index
  ) {
    return
  }

  if (hoveredIndex.value === index) {
    hoveredIndex.value = null
  }
}

function handleRowMouseLeave(): void {
  hoveredIndex.value = null
}

const rowInteractionListeners = mobile
  ? {
      contextmenu: handleContextMenu,
      pointerdown: handlePointerdown,
      pointerup: handlePointerUp,
      pointerout: handlePointerOut
    }
  : {
      click: handleOpenClick,
      mouseover: handleMouseOver,
      mouseout: handleMouseOut,
      mouseleave: handleRowMouseLeave
    }

watch(
  () => locationStore.highlightedIndex,
  (value) => {
    if (value !== null && value >= props.row.start && value <= props.row.end) {
      const highlightTimer = setTimeout(() => {
        locationStore.highlightedIndex = null
        highlightTimers.delete(highlightTimer)
      }, 2000)
      highlightTimers.add(highlightTimer)
    }
  }
)

onBeforeUnmount(() => {
  if (pressTimer !== null) {
    clearTimeout(pressTimer)
    pressTimer = null
  }
  highlightTimers.forEach((timer) => {
    clearTimeout(timer)
  })
  highlightTimers.clear()

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

<template>
  <div
    class="buffer-row-block position-relative w-100 d-flex flex-wrap no-select"
    v-on="rowInteractionListeners"
  >
    <DesktopHoverIcon
      v-if="!mobile && hoveredTile !== null"
      class="icon-hover"
      :index="hoveredTile.index"
      :left="hoveredTile.leftPixel"
      :top="hoveredTile.topPixel"
      :on-click="handleSelectClick"
    />
    <div
      v-for="tile in tiles"
      :key="tile.key"
      v-memo="[
        tile.tileClass,
        tile.tileStyle,
        tile.thumbnailClass,
        tile.pending,
        tile.durationLabel,
        tile.filenameLabel,
        tile.albumLabel,
        tile.chipMaxWidth,
        imagesDisabled
      ]"
      data-testid="gallery-item"
      :data-item-index="tile.index"
      role="button"
      aria-label="Open item"
      :class="tile.tileClass"
      :style="tile.tileStyle"
    >
      <ProcessingChip v-if="tile.pending" />
      <DurationChip v-if="tile.durationLabel !== null" :label="tile.durationLabel" />
      <FilenameChip
        v-if="tile.filenameLabel !== null"
        :label="tile.filenameLabel"
        :max-width="tile.chipMaxWidth"
      />
      <AlbumChip
        v-if="tile.albumLabel !== null"
        :label="tile.albumLabel"
        :max-width="tile.chipMaxWidth"
      />

      <img
        v-if="imagesDisabled"
        data-testid="open-item"
        class="thumbnail-image w-100 h-100 position-absolute"
        :data-item-index="tile.index"
        :src="transparentThumbnailSrc"
      />
      <img
        v-else
        v-thumbnail-src="tile.index"
        data-testid="open-item"
        :class="tile.thumbnailClass"
        :data-item-index="tile.index"
        decoding="async"
      />
    </div>
  </div>
</template>

<style scoped>
.gallery-tile::before,
.gallery-tile::after {
  content: '';
  position: absolute;
  inset: 0;
  pointer-events: none;
}

.gallery-tile::before {
  display: none;
  z-index: 3;
  height: 40px;
  background: linear-gradient(180deg, rgba(0, 0, 0, 0.5) 0%, rgba(255, 255, 255, 0) 100%);
}

.gallery-tile:not(.gallery-tile--mobile):hover::before {
  display: block;
}

.gallery-tile::after {
  z-index: 100;
  border: 4px solid transparent;
}

.gallery-tile--selected::after {
  border-color: rgb(var(--v-theme-primary));
}

.thumbnail-image {
  z-index: 2;
  object-fit: cover;
  pointer-events: none;
}

.thumbnail-image--album {
  border: 8px solid white;
}

.gallery-tile {
  background-repeat: no-repeat;
  background-size: 100% 100%;
  cursor: default;
}

.icon-hover {
  color: #fafafa;
  transition: color 0.3s;
  cursor: pointer;
}

.icon-hover:hover {
  color: white;
}

.locate-highlight::after {
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
