<script setup lang="ts">
import { computed } from 'vue'
import type { ReindexJobState, ReindexJobStatus, ReindexOperation } from '@/api/reindex'
import { reindexProgress } from '@/api/reindex'

const props = defineProps<{
  job: ReindexJobStatus
  canceling: boolean
}>()

const emit = defineEmits<{
  cancel: [jobId: string]
}>()

const labels: Record<ReindexJobState, string> = {
  queued: 'Queued',
  running: 'Running',
  completed: 'Completed',
  completedWithErrors: 'Completed with errors',
  canceled: 'Canceled',
  failed: 'Failed'
}

const colors: Record<ReindexJobState, string> = {
  queued: 'info',
  running: 'primary',
  completed: 'success',
  completedWithErrors: 'warning',
  canceled: 'secondary',
  failed: 'error'
}

const icons: Record<ReindexJobState, string> = {
  queued: 'mdi-clock-outline',
  running: 'mdi-progress-wrench',
  completed: 'mdi-check',
  completedWithErrors: 'mdi-alert-outline',
  canceled: 'mdi-cancel',
  failed: 'mdi-close'
}

const operationLabels: Record<ReindexOperation, string> = {
  exif: 'EXIF',
  dimensions: 'Dimensions',
  fileSize: 'File size',
  thumbnail: 'Thumbnail',
  visualHashes: 'Visual hashes',
  videoCompression: 'Video compression',
  clearTags: 'Clear tags'
}

const progress = computed(() => reindexProgress(props.job))
const stateLabel = computed(() => labels[props.job.state])
const stateColor = computed(() => colors[props.job.state])
const stateIcon = computed(() => icons[props.job.state])
const canCancel = computed(() => props.job.state === 'queued' || props.job.state === 'running')
const title = computed(() =>
  props.job.queuePosition === null
    ? stateLabel.value
    : `${stateLabel.value} · Queue #${props.job.queuePosition}`
)
const subtitle = computed(() => {
  const timestamp = new Date(props.job.startedAt ?? props.job.createdAt).toLocaleString()
  const targetLabel = props.job.total === 1 ? 'target' : 'targets'
  return `${timestamp} · ${props.job.total.toLocaleString()} ${targetLabel}`
})
const progressSummary = computed(
  () =>
    `${props.job.processed.toLocaleString()} of ${props.job.total.toLocaleString()} processed · ${progress.value}%`
)
const resultsSummary = computed(
  () =>
    `${props.job.succeeded.toLocaleString()} succeeded · ${props.job.failed.toLocaleString()} failed · ${props.job.skipped.toLocaleString()} skipped`
)
const operationsSummary = computed(() =>
  props.job.operations.map((operation) => operationLabels[operation]).join(' · ')
)
const shortId = (objectId: string) =>
  objectId === 'unknown' ? objectId : `${objectId.slice(0, 10)}…`
</script>

<template>
  <v-list border rounded="lg" bg-color="transparent" :lines="false" class="pa-0 mb-4">
    <v-list-item class="py-2">
      <template #prepend>
        <v-avatar :color="stateColor" variant="tonal" size="38">
          <v-icon :icon="stateIcon" size="20" />
        </v-avatar>
      </template>

      <v-list-item-title class="text-wrap">{{ title }}</v-list-item-title>
      <v-list-item-subtitle class="text-wrap">{{ subtitle }}</v-list-item-subtitle>

      <template #append>
        <v-btn
          v-if="canCancel"
          :text="job.cancelRequested ? 'Canceling…' : 'Cancel'"
          size="small"
          variant="tonal"
          color="warning"
          :loading="canceling"
          :disabled="job.cancelRequested"
          @click="emit('cancel', job.jobId)"
        />
      </template>
    </v-list-item>

    <v-divider />

    <v-list-item class="py-2">
      <template #prepend>
        <v-icon icon="mdi-progress-check" />
      </template>
      <v-list-item-title>Progress</v-list-item-title>
      <v-list-item-subtitle class="text-wrap">{{ progressSummary }}</v-list-item-subtitle>
      <v-progress-linear
        :model-value="progress"
        :indeterminate="job.state === 'queued'"
        :color="stateColor"
        rounded
        height="6"
        class="mt-2"
      />
    </v-list-item>

    <v-divider />

    <v-list-item prepend-icon="mdi-chart-box-outline" class="py-2">
      <v-list-item-title>Results</v-list-item-title>
      <v-list-item-subtitle class="text-wrap">{{ resultsSummary }}</v-list-item-subtitle>
    </v-list-item>

    <v-divider />

    <v-list-item prepend-icon="mdi-tools" class="py-2">
      <v-list-item-title>Operations</v-list-item-title>
      <v-list-item-subtitle class="text-wrap">{{ operationsSummary }}</v-list-item-subtitle>
    </v-list-item>

    <template v-if="job.errors.length > 0">
      <v-divider />
      <v-list-subheader title="Recent errors" color="error" />
      <v-divider />

      <template v-for="(error, index) in job.errors" :key="`${error.objectId}-${error.stage}`">
        <v-list-item class="py-2">
          <template #prepend>
            <v-icon icon="mdi-alert-circle-outline" color="error" />
          </template>
          <v-list-item-title class="text-wrap">{{ error.stage }}</v-list-item-title>
          <v-list-item-subtitle class="text-wrap text-break">
            {{ shortId(error.objectId) }} · {{ error.message }}
          </v-list-item-subtitle>
        </v-list-item>
        <v-divider v-if="index < job.errors.length - 1" />
      </template>
    </template>
  </v-list>
</template>
