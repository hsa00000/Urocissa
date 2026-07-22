<script setup lang="ts">
import { computed } from 'vue'
import type { UploadStatus } from '@/store/uploadStore'

const props = defineProps<{
  status: UploadStatus
  loaded: number
  total: number
}>()

const progress = computed(() => {
  if (props.total <= 0) return 0
  return Math.min(100, Math.round((props.loaded / props.total) * 100))
})

const statusLabel = computed(() => {
  switch (props.status) {
    case 'pending':
      return 'Pending'
    case 'uploading':
      return 'Uploading'
    case 'processing':
      return 'Processing'
    case 'success':
      return 'Completed'
    case 'error':
      return 'Failed'
    case 'canceled':
      return 'Canceled'
  }
})

const statusColor = computed(() => {
  switch (props.status) {
    case 'success':
      return 'success'
    case 'error':
      return 'error'
    case 'canceled':
      return 'warning'
    case 'processing':
      return 'secondary'
    case 'uploading':
      return 'primary'
    case 'pending':
      return 'grey'
  }
})

const statusIcon = computed(() => {
  switch (props.status) {
    case 'success':
      return 'mdi-check'
    case 'error':
      return 'mdi-alert-circle-outline'
    case 'canceled':
      return 'mdi-cancel'
    case 'processing':
      return 'mdi-cog-outline'
    case 'uploading':
      return 'mdi-upload'
    case 'pending':
      return 'mdi-clock-outline'
  }
})
</script>

<template>
  <v-progress-linear
    v-if="status === 'uploading'"
    :model-value="progress"
    color="primary"
    height="20"
    rounded
    striped
  >
    <template #default>
      <span class="upload-progress-label text-caption font-weight-medium">{{ progress }}%</span>
    </template>
  </v-progress-linear>

  <v-progress-linear
    v-else-if="status === 'processing'"
    color="secondary"
    height="20"
    indeterminate
    rounded
  >
    <template #default>
      <span class="upload-progress-label text-caption font-weight-medium">Processing</span>
    </template>
  </v-progress-linear>

  <v-chip v-else :color="statusColor" size="small" label>
    <v-icon start :icon="statusIcon" />
    {{ statusLabel }}
  </v-chip>
</template>

<style scoped>
.upload-progress-label {
  color: white;
  text-shadow: 0 0 2px rgba(0, 0, 0, 0.55);
}
</style>
