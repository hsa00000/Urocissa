<template>
  <div
    v-if="abstractData && abstractData.database"
    id="col-ref"
    class="h-100 d-flex align-center justify-center"
  >
    <DisplayDatabaseImage
      :key="index"
      v-if="abstractData.database.extType === 'image'"
      :isolation-id="isolationId"
      :index="index"
      :abstract-data="abstractData"
    />

    <DisplayDatabaseVideo
      :key="index"
      v-if="abstractData.database.extType === 'video' && !abstractData.database.pending"
      :database="abstractData.database"
      :hash="abstractData.database.hash"
      :isolation-id="isolationId"
      :enable-watch="enableWatch"
    />
    <v-card
      v-if="abstractData.database.extType === 'video' && abstractData.database.pending"
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
import { AbstractData, IsolationId } from '@type/types'

import DisplayDatabaseVideo from './DisplayDatabaseVideo.vue'
import DisplayDatabaseImage from './DisplayDatabaseImage.vue'

defineProps<{
  isolationId: IsolationId
  hash: string
  index: number
  abstractData: AbstractData
  enableWatch: boolean
}>()
</script>
