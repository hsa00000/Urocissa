<template>
  <div class="h-100 w-100 position-relative">
    <NavigationOverlays
      :previous-hash="previousHash"
      :next-hash="nextHash"
      :previous-page="previousPage"
      :next-page="nextPage"
      :show="!configStore.isMobile"
    />
    <div class="h-100 w-100">
      <ViewPageDisplayDatabase
        v-if="
          abstractData &&
          (abstractData.type === 'image' || abstractData.type === 'video') &&
          !configStore.disableImg
        "
        :index="index"
        :hash="hash"
        :abstract-data="abstractData"
        :isolation-id="isolationId"
        :enable-watch="true"
      />
      <ViewPageDisplayAlbum
        v-if="abstractData && abstractData.type === 'album' && !configStore.disableImg"
        :index="index"
        :album="abstractData"
      />
    </div>
  </div>
</template>

<script setup lang="ts">
import { useConfigStore } from '@/store/configStore'
import ViewPageDisplayDatabase from './DisplayDatabase.vue'
import ViewPageDisplayAlbum from './DisplayAlbum.vue'
import NavigationOverlays from './NavigationOverlays.vue'
import type { EnrichedUnifiedData, IsolationId } from '@type/types'

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

const configStore = useConfigStore(props.isolationId)
</script>

<style scoped>
/* 以容器（#image-display-col）為查詢上下文 */
.nav-btn {
  position: absolute;
  top: 50%;
  transform: translateY(-50%);
  z-index: 1;
  /* 觸控/滑鼠友善的點擊面積 */
  inline-size: 48px;
  block-size: 50%;
  /* 去除卡片背景/陰影干擾（Vuetify 可按需調整） */
  box-shadow: none;
}

.nav-left {
  left: 0;
}
.nav-right {
  right: 0;
}

/* 窄容器時，縮小導覽按鈕的高度 */
@container image-col (max-width: 600px) {
  .nav-btn {
    block-size: 40%;
  }
}
</style>
