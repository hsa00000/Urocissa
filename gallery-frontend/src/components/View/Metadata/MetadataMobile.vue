<template>
  <div class="swiper-container h-100 w-100">
    <swiper
      :modules="modules"
      :slides-per-view="1"
      :space-between="10"
      :centered-slides="true"
      :initial-slide="currentSlideIndex"
      :resistance="true"
      :resistance-ratio="0.3"
      :allow-touch-move="true"
      @slide-change="onSlideChange"
      @swiper="onSwiper"
      class="h-100"
    >
      <swiper-slide v-if="previousHash !== undefined">
        <div class="slide-content">
          <MetadataContent
            v-if="previousAbstractData"
            :abstract-data="previousAbstractData"
            :index="index - 1"
            :hash="previousHash"
            :isolation-id="isolationId"
          />
        </div>
      </swiper-slide>

      <swiper-slide>
        <div class="slide-content">
          <MetadataContent
            v-if="abstractData"
            :abstract-data="abstractData"
            :index="index"
            :hash="hash"
            :isolation-id="isolationId"
          />
        </div>
      </swiper-slide>

      <swiper-slide v-if="nextHash !== undefined">
        <div class="slide-content">
          <MetadataContent
            v-if="nextAbstractData"
            :abstract-data="nextAbstractData"
            :index="index + 1"
            :hash="nextHash"
            :isolation-id="isolationId"
          />
        </div>
      </swiper-slide>
    </swiper>
  </div>
</template>

<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { Swiper, SwiperSlide } from 'swiper/vue'
import { Manipulation } from 'swiper/modules'
import type { Swiper as SwiperType } from 'swiper'
import { useDataStore } from '@/store/dataStore'
import type { EnrichedUnifiedData, IsolationId } from '@type/types'
import MetadataContent from './MetadataContent.vue'
import 'swiper/css'
import 'swiper/css/manipulation'

const props = defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData
}>()

const dataStore = useDataStore(props.isolationId)
const route = useRoute()
const router = useRouter()

const modules = [Manipulation]
const swiperInstance = shallowRef<SwiperType | null>(null)

const nextAbstractData = computed(() => dataStore.data.get(props.index + 1))
const previousAbstractData = computed(() => dataStore.data.get(props.index - 1))

const nextHash = computed(() => nextAbstractData.value?.id)

const previousHash = computed(() => previousAbstractData.value?.id)

const currentSlideIndex = computed(() => (previousHash.value !== undefined ? 1 : 0))

function onSwiper(swiper: SwiperType) {
  swiperInstance.value = swiper
}

function onSlideChange(swiper: SwiperType) {
  const currentIndex = swiper.activeIndex
  const hasPrevious = previousHash.value !== undefined
  const hasNext = nextHash.value !== undefined

  if (hasPrevious) {
    if (currentIndex === 0 && previousHash.value) {
      navigateToHash(previousHash.value)
    } else if (currentIndex === 2 && hasNext && nextHash.value) {
      navigateToHash(nextHash.value)
    }
  } else if (currentIndex === 1 && hasNext && nextHash.value) {
    navigateToHash(nextHash.value)
  }
}

function navigateToHash(targetHash: string) {
  if (route.meta.level === 2) {
    const updatedParams = { ...route.params, hash: targetHash }
    void router.replace({
      name: route.name ?? undefined,
      params: updatedParams,
      query: route.query
    })
  } else if (route.meta.level === 4) {
    const updatedParams = { ...route.params, subhash: targetHash }
    void router.replace({
      name: route.name ?? undefined,
      params: updatedParams,
      query: route.query
    })
  }
}

watch(
  () => props.index,
  () => {
    if (swiperInstance.value) {
      swiperInstance.value.slideTo(currentSlideIndex.value, 0)
    }
  }
)
</script>

<style scoped>
.swiper-container {
  width: 100%;
  height: 100%;
  overflow: hidden;
  touch-action: pan-y;
}

.slide-content {
  width: 100%;
  height: 100%;
  min-width: 0;
  min-height: 0;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}

:deep(.swiper) {
  width: 100%;
  height: 100%;
}

:deep(.swiper-slide) {
  background: transparent;
  display: flex;
  flex-direction: column;
  min-width: 0;
  min-height: 0;
  overflow: hidden;
}
</style>
