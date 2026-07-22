<script setup lang="ts">
import { computed } from 'vue'
import type { ReindexJobStatus } from '@/api/reindex'
import ReindexJobItem from './ReindexJobItem.vue'

const props = defineProps<{
  jobs: ReindexJobStatus[]
  loading: boolean
  cancelingJobIds: ReadonlySet<string>
}>()

const emit = defineEmits<{
  cancel: [jobId: string]
}>()

const activeJobCount = computed(
  () => props.jobs.filter((job) => job.state === 'queued' || job.state === 'running').length
)

const jobsSummary = computed(() => {
  const jobLabel = props.jobs.length === 1 ? 'job' : 'jobs'
  const activeLabel = activeJobCount.value === 0 ? 'No active jobs' : `${activeJobCount.value} active`
  return `${props.jobs.length.toLocaleString()} ${jobLabel} · ${activeLabel}`
})
</script>

<template>
  <v-sheet color="transparent">
    <v-sheet
      v-if="loading && jobs.length === 0"
      color="transparent"
      class="d-flex justify-center py-12"
    >
      <v-progress-circular indeterminate color="primary" />
    </v-sheet>

    <v-empty-state
      v-else-if="jobs.length === 0"
      icon="mdi-tray-remove"
      title="No reindex jobs"
      text="Create a job to see its queue position and progress here."
    />

    <v-sheet v-else color="transparent">
      <v-list bg-color="transparent" class="pa-0 mb-4">
        <v-list-item title="Reindex jobs" :subtitle="jobsSummary" lines="two" class="px-0">
          <template #prepend>
            <v-avatar color="primary" variant="tonal" size="42">
              <v-icon icon="mdi-format-list-checks" />
            </v-avatar>
          </template>
        </v-list-item>
      </v-list>

      <ReindexJobItem
        v-for="job in jobs"
        :key="job.jobId"
        :job="job"
        :canceling="cancelingJobIds.has(job.jobId)"
        @cancel="emit('cancel', $event)"
      />
    </v-sheet>
  </v-sheet>
</template>
