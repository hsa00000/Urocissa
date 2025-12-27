<template>
  <div
    v-if="abstractData && (abstractData.type === 'image' || abstractData.type === 'video')"
    id="col-ref"
    class="h-100 d-flex align-center justify-center"
  >
    <DisplayDatabaseImage
      :key="index"
      v-if="abstractData.type === 'image'"
      :isolation-id="isolationId"
      :index="index"
      :abstract-data="abstractData"
    />

    <DisplayDatabaseVideo
      :key="index"
      v-if="abstractData.type === 'video' && !abstractData.pending"
      :database="abstractData"
      :hash="abstractData.id"
      :isolation-id="isolationId"
      :enable-watch="enableWatch"
    />
    <v-card
      v-if="abstractData.type === 'video' && abstractData.pending"
      class="d-flex align-center justify-start"
      outlined
      style="padding: 16px"
    >
      <div class="d-flex align-center">
        <v-icon size="48" color="warning">mdi-alert-circle-outline</v-icon>
      </div>
      <div class="text-left pl-4">
        <div>This video is currently being processed.</div>
        <div>Please check back later.</div>
      </div>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import { EnrichedUnifiedData, IsolationId } from '@type/types'

import DisplayDatabaseVideo from './DisplayDatabaseVideo.vue'
import DisplayDatabaseImage from './DisplayDatabaseImage.vue'

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: EnrichedUnifiedData
  enableWatch: boolean
}>()
</script>
