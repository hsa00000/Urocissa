<template>
  <div v-if="abstractData" id="col-ref" class="h-100 d-flex align-center justify-center">
    <DisplayImage
      v-if="abstractData.data.type === 'image'"
      :key="index"
      :isolation-id="isolationId"
      :index="index"
      :data="abstractData.data"
    />

    <DisplayVideo
      v-else-if="abstractData.data.type === 'video' && !abstractData.data.pending"
      :key="index"
      :data="abstractData.data"
      :isolation-id="isolationId"
      :enable-watch="enableWatch"
    />

    <v-card
      v-else-if="abstractData.data.type === 'video' && abstractData.data.pending"
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
import type { AbstractData, IsolationId } from '@type/types'
import DisplayVideo from './DisplayVideo.vue'
import DisplayImage from './DisplayImage.vue'

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData
  enableWatch: boolean
}>()
</script>
