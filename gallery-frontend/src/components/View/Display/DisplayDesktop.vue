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
        :isolation-id="isolationId"
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

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData | undefined
  previousHash: string | undefined
  nextHash: string | undefined
  previousPage: Record<string, unknown> | undefined
  nextPage: Record<string, unknown> | undefined
}>()

const configStore = useConfigStore('mainId')
</script>
