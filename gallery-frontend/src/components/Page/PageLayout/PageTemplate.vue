<template>
  <HomeMainBar />
  <Drawer />

  <div class="page-root" :style="{ height: `calc(100% - ${navBarHeight}px)` }">
    <slot name="overlay"></slot>

    <v-container
      v-if="resolved.ready"
      :id="resolved.containerId"
      :class="wrapperContainerClass"
      fluid
    >
      <v-row justify="center" :class="wrapperRowClass">
        <v-col
          :cols="resolved.col.cols"
          :sm="resolved.col.sm"
          :md="resolved.col.md"
          :lg="resolved.col.lg"
          :class="wrapperColClass"
        >
          <v-card tile flat :class="wrapperCardClass">
            <slot name="content"></slot>
          </v-card>
        </v-col>
      </v-row>
    </v-container>

    <v-container
      v-else
      fluid
      class="fill-height d-flex align-center justify-center bg-surface-light"
    >
      <slot name="loading">
        <v-progress-circular indeterminate />
      </slot>
    </v-container>
  </div>
</template>

<script setup lang="ts">
import { computed, provide, ref, onMounted, onUnmounted } from 'vue'
import { onBeforeRouteLeave } from 'vue-router'
import HomeMainBar from '@/components/NavBar/HomeBars/HomeMainBar.vue'
import Drawer from './Drawer.vue'
import { useCollectionStore } from '@/store/collectionStore'
import { navBarHeight } from '@/type/constants'

interface PageCol {
  cols?: number
  sm?: number
  md?: number
  lg?: number
}

type Preset = 'full' | 'card'
type Width = 'pane' | 'narrow' | 'compact' | 'default' | 'wide' | 'full'

interface PresetConfig {
  containerId: string
  containerClass: string | string[]
  cardClass: string | string[]
  col: PageCol
  fillHeight: boolean
  centerContent: boolean
  colClass: string | string[]
}

const baseContainerClass = 'h-100 w-100 pa-0 min-h-0'
const baseCardClass = 'overflow-y-auto w-100'

const presetDefaults = {
  full: {
    containerId: 'home-container',
    containerClass: [baseContainerClass, 'overflow-hidden'],
    cardClass: baseCardClass,
    col: { cols: 12, sm: 12, md: 12, lg: 12 },
    fillHeight: true,
    centerContent: false,
    colClass: 'pa-0'
  },
  card: {
    containerId: 'table-container',
    containerClass: [baseContainerClass, 'bg-surface-light', 'overflow-y-auto'],
    cardClass: baseCardClass,
    col: { cols: 12, sm: 12, md: 10, lg: 8 },
    fillHeight: false,
    centerContent: true,
    colClass: ''
  }
} satisfies Record<Preset, PresetConfig>

const widthCols: Record<Width, PageCol> = {
  pane: { cols: 12, sm: 6, md: 4, lg: 3 },

  // 單頁卡片：到 md 才開始變窄（平板 sm 多數仍滿寬可讀性較好）
  narrow: { cols: 12, sm: 12, md: 8, lg: 6 },

  // 你要的「default 與 narrow 之間」
  compact: { cols: 12, sm: 12, md: 9, lg: 7 },

  default: { cols: 12, sm: 12, md: 10, lg: 8 },
  wide: { cols: 12, sm: 12, md: 11, lg: 10 },
  full: { cols: 12, sm: 12, md: 12, lg: 12 }
}

const props = withDefaults(
  defineProps<{
    preset?: Preset
    width?: Width
    ready?: boolean
    containerId?: string
    containerClass?: string | string[]
    cardClass?: string | string[]
    col?: PageCol
    fillHeight?: boolean
    centerContent?: boolean
    alwaysShowVerticalScrollbar?: boolean
    /** extra class for v-col wrapper (Home needs pa-0) */
    colClass?: string | string[]
  }>(),
  {
    preset: 'full',
    width: undefined,
    ready: true,
    containerId: undefined,
    containerClass: undefined,
    cardClass: undefined,
    col: undefined,
    fillHeight: undefined,
    centerContent: undefined,
    alwaysShowVerticalScrollbar: false,
    colClass: undefined
  }
)

const showDrawer = ref(false)
const collectionStore = useCollectionStore('mainId')
provide('showDrawer', showDrawer)

const exitEditMode = () => {
  if (collectionStore.editModeOn) {
    collectionStore.editModeOn = false
    return true
  }
  return false
}

const handleKeydown = (event: KeyboardEvent) => {
  if (event.key === 'Escape') collectionStore.leaveEdit()
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown)
})

onBeforeRouteLeave(() => {
  if (exitEditMode()) return false
})

const preset = computed(() => presetDefaults[props.preset])

function normalizeCol(col: PageCol): Required<PageCol> {
  return {
    cols: col.cols ?? 12,
    sm: col.sm ?? 12,
    md: col.md ?? 12,
    lg: col.lg ?? 12
  }
}

const resolved = computed(() => {
  const p = preset.value
  const effectiveCol = props.col ?? (props.width ? widthCols[props.width] : p.col)
  return {
    ready: props.ready,
    containerId: props.containerId ?? p.containerId,
    containerClass: props.containerClass ?? p.containerClass,
    cardClass: props.cardClass ?? p.cardClass,
    col: normalizeCol(effectiveCol),
    fillHeight: props.fillHeight ?? p.fillHeight,
    centerContent: props.centerContent ?? p.centerContent,
    colClass: props.colClass ?? p.colClass
  }
})

const wrapperRowClass = computed(() => {
  const cls: string[] = ['ma-0', 'w-100']
  if (resolved.value.fillHeight) cls.push('h-100')
  return cls
})

const wrapperContainerClass = computed(() => {
  const base = Array.isArray(resolved.value.containerClass)
    ? [...resolved.value.containerClass]
    : [resolved.value.containerClass]
  if (props.alwaysShowVerticalScrollbar) base.push('overflow-y-scroll')
  return base
})

const wrapperColClass = computed(() => {
  const cls: string[] = ['d-flex', 'w-100']
  cls.push(resolved.value.centerContent ? 'justify-center' : 'justify-start')
  if (resolved.value.fillHeight) cls.push('h-100')

  const colClass = resolved.value.colClass
  if (Array.isArray(colClass)) {
    cls.push(...colClass.filter((x) => x !== ''))
  } else if (colClass !== '') {
    cls.push(colClass)
  }

  return cls
})

const wrapperCardClass = computed(() => {
  const base = Array.isArray(resolved.value.cardClass)
    ? [...resolved.value.cardClass]
    : [resolved.value.cardClass]
  if (resolved.value.fillHeight) base.push('h-100')
  return base
})
</script>

<style scoped>
#table-container {
  display: flex;
  justify-content: center;
  position: relative;
  padding: 4px;
  padding-top: 4px;
  height: 100%;
  width: 100%;
  min-height: 0;
}
</style>
