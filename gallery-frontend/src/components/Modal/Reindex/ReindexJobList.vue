<template>
  <div>
    <div v-if="loading && jobs.length === 0" class="d-flex justify-center py-12">
      <v-progress-circular indeterminate color="primary" />
    </div>

    <v-empty-state
      v-else-if="jobs.length === 0"
      icon="mdi-tray-remove"
      title="No reindex jobs"
      text="Create a job to see its queue position and progress here."
    />

    <div v-else class="d-flex flex-column ga-3">
      <v-card v-for="job in jobs" :key="job.jobId" variant="outlined" class="job-card">
        <v-card-item>
          <template #prepend>
            <v-avatar :color="stateColor(job.state)" variant="tonal" size="38">
              <v-icon :icon="stateIcon(job.state)" size="20" />
            </v-avatar>
          </template>
          <v-card-title class="text-subtitle-1">
            {{ stateLabel(job.state) }}
            <span v-if="job.queuePosition !== null" class="text-medium-emphasis">
              · Queue #{{ job.queuePosition }}
            </span>
          </v-card-title>
          <v-card-subtitle>
            {{ formatTimestamp(job.startedAt ?? job.createdAt) }} · {{ job.total.toLocaleString() }}
            targets
          </v-card-subtitle>
          <template #append>
            <v-btn
              v-if="canCancel(job)"
              size="small"
              variant="tonal"
              color="warning"
              :loading="cancelingJobIds.has(job.jobId)"
              :disabled="job.cancelRequested"
              @click="emit('cancel', job.jobId)"
            >
              {{ job.cancelRequested ? 'Canceling…' : 'Cancel' }}
            </v-btn>
          </template>
        </v-card-item>

        <v-card-text>
          <v-progress-linear
            :model-value="reindexProgress(job)"
            :indeterminate="job.state === 'queued'"
            :color="stateColor(job.state)"
            rounded
            height="7"
            class="mb-3"
          />

          <div class="d-flex flex-wrap ga-2 mb-3">
            <v-chip size="small" color="success" variant="tonal">
              {{ job.succeeded }} succeeded
            </v-chip>
            <v-chip size="small" color="error" variant="tonal">
              {{ job.failed }} failed
            </v-chip>
            <v-chip size="small" variant="tonal"> {{ job.skipped }} skipped </v-chip>
            <v-chip size="small" variant="text"> {{ job.processed }}/{{ job.total }} processed </v-chip>
          </div>

          <div class="d-flex flex-wrap ga-1">
            <v-chip
              v-for="operation in job.operations"
              :key="operation"
              size="x-small"
              variant="outlined"
            >
              {{ operationLabel(operation) }}
            </v-chip>
          </div>

          <v-alert
            v-if="job.errors.length > 0"
            type="error"
            variant="tonal"
            density="compact"
            class="mt-3"
          >
            <div class="font-weight-medium mb-1">Recent errors</div>
            <div v-for="error in job.errors" :key="`${error.objectId}-${error.stage}`" class="error-row">
              <code>{{ shortId(error.objectId) }}</code> · {{ error.stage }} — {{ error.message }}
            </div>
          </v-alert>
        </v-card-text>
      </v-card>
    </div>
  </div>
</template>

<script setup lang="ts">
import type {
  ReindexJobState,
  ReindexJobStatus,
  ReindexOperation
} from '@/api/reindex'
import { reindexProgress } from '@/api/reindex'

defineProps<{
  jobs: ReindexJobStatus[]
  loading: boolean
  cancelingJobIds: ReadonlySet<string>
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

const stateLabel = (state: ReindexJobState) => labels[state]
const stateColor = (state: ReindexJobState) => colors[state]
const stateIcon = (state: ReindexJobState) => icons[state]
const operationLabel = (operation: ReindexOperation) => operationLabels[operation]
const canCancel = (job: ReindexJobStatus) => job.state === 'queued' || job.state === 'running'
const shortId = (objectId: string) =>
  objectId === 'unknown' ? objectId : `${objectId.slice(0, 10)}…`
const formatTimestamp = (timestamp: number) => new Date(timestamp).toLocaleString()
</script>

<style scoped>
.job-card {
  border-color: rgba(255, 255, 255, 0.14);
}

.error-row {
  overflow-wrap: anywhere;
  font-size: 0.78rem;
  line-height: 1.45;
}
</style>
