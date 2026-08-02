<template>
  <div
    id="abstractData-col"
    v-if="abstractData"
    class="h-100 flex-grow-0 flex-shrink-0 bg-surface"
    :style="{
      width: constStore.showInfo ? undefined : '0',
      zIndex: 1
    }"
  >
    <MetadataMobile
      v-if="configStore.isMobile"
      :abstract-data="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
    <MetadataContent
      v-else
      :abstract-data="abstractData"
      :index="index"
      :hash="hash"
      :isolation-id="isolationId"
    />
  </div>
</template>

<script setup lang="ts">
import { useConstStore } from '@/store/constStore'
import { useConfigStore } from '@/store/configStore'
import { EnrichedUnifiedData, IsolationId } from '@type/types'
import MetadataContent from './MetadataContent.vue'
import MetadataMobile from './MetadataMobile.vue'

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData
}>()

const constStore = useConstStore('mainId')
const configStore = useConfigStore('mainId')
</script>

<style scoped>
#abstractData-col {
  width: 360px;
}
@media (width <= 720px) {
  #abstractData-col {
    width: 100%;
  }
}
</style>
