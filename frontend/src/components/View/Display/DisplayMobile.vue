<template>
  <div class="swiper-container h-100 w-100">
    <swiper
      v-if="abstractData && abstractData.type === 'image'"
      :modules="modules"
      :slides-per-view="1"
      :space-between="10"
      :centered-slides="true"
      :initial-slide="currentSlideIndex"
      :resistance="true"
      :resistance-ratio="0.3"
      :allow-touch-move="canHandleNav()"
      :zoom="true"
      @slide-change="onSlideChange"
      @swiper="onSwiper"
      @slider-first-move="onDragStart"
      @touch-end="onTouchEnd"
      @transition-end="onTransitionEnd"
      @zoom-change="onZoomChange"
      class="h-100"
    >
      <swiper-slide v-if="previousHash !== undefined">
        <div class="swiper-zoom-container">
          <div class="slide-content">
            <ViewPageDisplayDatabase
              v-if="
                previousAbstractData &&
                (previousAbstractData.type === 'image' || previousAbstractData.type === 'video') &&
                !settingsStore.disableImg
              "
              :index="index - 1"
              :hash="previousAbstractData.id"
              :abstract-data="previousAbstractData"
              :isolation-id="isolationId"
              :enable-watch="false"
            />
            <ViewPageDisplayAlbum
              v-if="
                previousAbstractData &&
                previousAbstractData.type === 'album' &&
                !settingsStore.disableImg
              "
              :index="index - 1"
              :album="previousAbstractData"
            />
          </div>
        </div>
      </swiper-slide>

      <swiper-slide>
        <div class="swiper-zoom-container">
          <div class="slide-content">
            <ViewPageDisplayDatabase
              v-if="
                abstractData &&
                ['image', 'video'].includes(abstractData.type) &&
                !settingsStore.disableImg
              "
              :index="index"
              :hash="hash"
              :abstract-data="abstractData"
              :isolation-id="isolationId"
              :enable-watch="false"
            />
            <ViewPageDisplayAlbum
              v-if="
                abstractData && ['album'].includes(abstractData.type) && !settingsStore.disableImg
              "
              :index="index"
              :album="abstractData as unknown as GalleryAlbum"
            />
          </div>
        </div>
      </swiper-slide>

      <swiper-slide v-if="nextHash !== undefined">
        <div class="swiper-zoom-container">
          <div class="slide-content">
            <ViewPageDisplayDatabase
              v-if="
                nextAbstractData &&
                (nextAbstractData.type === 'image' || nextAbstractData.type === 'video') &&
                !settingsStore.disableImg
              "
              :index="index + 1"
              :hash="nextAbstractData.id"
              :abstract-data="nextAbstractData"
              :isolation-id="isolationId"
              :enable-watch="false"
            />
            <ViewPageDisplayAlbum
              v-if="
                nextAbstractData && nextAbstractData.type === 'album' && !settingsStore.disableImg
              "
              :index="index + 1"
              :album="nextAbstractData"
            />
          </div>
        </div>
      </swiper-slide>
    </swiper>
    <swiper
      v-else
      :modules="modules"
      :slides-per-view="1"
      :space-between="10"
      :centered-slides="true"
      :initial-slide="currentSlideIndex"
      :resistance="true"
      :resistance-ratio="0.3"
      :allow-touch-move="canHandleNav()"
      @slide-change="onSlideChange"
      @swiper="onSwiper"
      class="h-100"
    >
      <!-- 前一張 -->
      <swiper-slide v-if="previousHash !== undefined">
        <div class="slide-content">
          <ViewPageDisplayDatabase
            v-if="
              previousAbstractData &&
              (previousAbstractData.type === 'image' || previousAbstractData.type === 'video') &&
              !settingsStore.disableImg
            "
            :index="index - 1"
            :hash="previousAbstractData.id"
            :abstract-data="previousAbstractData"
            :isolation-id="isolationId"
            :enable-watch="false"
          />
          <ViewPageDisplayAlbum
            v-if="
              previousAbstractData &&
              previousAbstractData.type === 'album' &&
              !settingsStore.disableImg
            "
            :index="index - 1"
            :album="previousAbstractData"
          />
        </div>
      </swiper-slide>

      <!-- 目前這張 -->
      <swiper-slide>
        <div class="slide-content">
          <ViewPageDisplayDatabase
            v-if="
              abstractData &&
              ['image', 'video'].includes(abstractData.type) &&
              !settingsStore.disableImg
            "
            :index="index"
            :hash="hash"
            :abstract-data="abstractData"
            :isolation-id="isolationId"
            :enable-watch="true"
          />
          <ViewPageDisplayAlbum
            v-if="
              abstractData && ['album'].includes(abstractData.type) && !settingsStore.disableImg
            "
            :index="index"
            :album="abstractData as GalleryAlbum"
          />
        </div>
      </swiper-slide>

      <!-- 下一張 -->
      <swiper-slide v-if="nextHash !== undefined">
        <div class="slide-content">
          <ViewPageDisplayDatabase
            v-if="
              nextAbstractData &&
              (nextAbstractData.type === 'image' || nextAbstractData.type === 'video') &&
              !settingsStore.disableImg
            "
            :index="index + 1"
            :hash="nextAbstractData.id"
            :abstract-data="nextAbstractData"
            :isolation-id="isolationId"
            :enable-watch="false"
          />
          <ViewPageDisplayAlbum
            v-if="
              nextAbstractData && nextAbstractData.type === 'album' && !settingsStore.disableImg
            "
            :index="index + 1"
            :album="nextAbstractData"
          />
        </div>
      </swiper-slide>
    </swiper>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
import ViewPageDisplayDatabase from './DisplayDatabase.vue'
import ViewPageDisplayAlbum from './DisplayAlbum.vue'
import { useSettingsStore } from '@/store/settingsStore'
import { useModalStore } from '@/store/modalStore'
import { Manipulation, Zoom } from 'swiper/modules'
import 'swiper/css'
import 'swiper/css/manipulation'
import 'swiper/css/zoom'
import type { Swiper as SwiperType } from 'swiper'
import type { EnrichedUnifiedData, IsolationId, GalleryAlbum } from '@type/types'
import { Swiper, SwiperSlide } from 'swiper/vue'

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData | undefined
  previousHash: string | undefined
  nextHash: string | undefined
  previousPage: Record<string, unknown> | undefined
  nextPage: Record<string, unknown> | undefined
}>()

const settingsStore = useSettingsStore(props.isolationId)
const dataStore = useDataStore(props.isolationId)
const route = useRoute()
const router = useRouter()

const modules = [Manipulation, Zoom]
const swiperInstance = ref<SwiperType | null>(null)

const nextAbstractData = computed(() => dataStore.data.get(props.index + 1))
const previousAbstractData = computed(() => dataStore.data.get(props.index - 1))

const currentSlideIndex = computed(() => (props.previousHash !== undefined ? 1 : 0))

function canHandleNav(): boolean {
  const modalStore = useModalStore('mainId')
  return (
    settingsStore.isMobile &&
    ((route.meta.level === 2 && props.isolationId === 'mainId') ||
      (route.meta.level === 4 && props.isolationId === 'subId')) &&
    !modalStore.showEditTagsModal
  )
}

function onSwiper(swiper: SwiperType) {
  swiperInstance.value = swiper
}

function onDragStart(swiper: SwiperType) {
  if (swiper.zoom.enabled) swiper.zoom.disable()
}

function onTouchEnd(swiper: SwiperType) {
  if (!swiper.zoom.enabled) swiper.zoom.enable()
}

function onTransitionEnd(swiper: SwiperType) {
  if (!swiper.zoom.enabled) swiper.zoom.enable()
}

function onZoomChange(swiper: SwiperType, scale: number) {
  swiper.allowTouchMove = scale === 1
}

function onSlideChange(swiper: SwiperType) {
  if (swiper.zoom.scale !== 1) swiper.disable()
  else swiper.enable()

  if (!canHandleNav()) return

  const currentIndex = swiper.activeIndex
  const hasPrevious = props.previousHash !== undefined
  const hasNext = props.nextHash !== undefined

  if (hasPrevious) {
    if (currentIndex === 0 && props.previousPage) {
      router.replace(props.previousPage).catch((err: unknown) => {
        console.error(err)
      })
    } else if (currentIndex === 2 && hasNext && props.nextPage) {
      router.replace(props.nextPage).catch((err: unknown) => {
        console.error(err)
      })
    }
  } else {
    if (currentIndex === 1 && hasNext && props.nextPage) {
      router.replace(props.nextPage).catch((err: unknown) => {
        console.error(err)
      })
    }
  }
}

watch(
  () => props.index,
  () => {
    if (swiperInstance.value && settingsStore.isMobile) {
      swiperInstance.value.slideTo(currentSlideIndex.value, 0)
    }
  }
)
</script>

<style scoped>
.swiper-container {
  width: 100%;
  overflow: hidden;
  touch-action: none;
}
.slide-content {
  width: 100%;
  height: 100%;
  display: flex;
  align-items: center;
  justify-content: center;
}
:deep(.swiper) {
  width: 100%;
  height: 100%;
}
:deep(.swiper-slide) {
  text-align: center;
  font-size: 18px;
  background: transparent;
  display: flex;
  justify-content: center;
  align-items: center;
}
:deep(.swiper-zoom-container) {
  width: 100%;
  height: 100%;
  display: flex;
  justify-content: center;
  align-items: center;
}
</style>
