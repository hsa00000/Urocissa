<template>
  <v-form @submit.prevent="submitPlan">
    <div class="d-flex align-center ga-3 mb-4">
      <v-avatar color="primary" variant="tonal" size="42">
        <v-icon icon="mdi-image-sync-outline" />
      </v-avatar>
      <div>
        <div class="text-subtitle-1 font-weight-bold">Choose what to rebuild</div>
        <div class="text-body-2 text-medium-emphasis">
          {{ targetCount.toLocaleString() }} selected {{ targetCount === 1 ? 'object' : 'objects' }}
        </div>
      </div>
    </div>

    <section v-for="group in optionGroups" :key="group.title" class="mb-5">
      <div class="text-overline text-medium-emphasis mb-2">{{ group.title }}</div>
      <div class="d-flex flex-column ga-2">
        <ReindexOptionRow
          v-for="option in group.options"
          :key="option.operation"
          :model-value="hasOperation(option.operation)"
          :title="option.title"
          :description="option.description"
          :applicability="option.applicability"
          :danger="option.danger"
          @update:model-value="setOperation(option.operation, $event)"
        />
      </div>
    </section>

    <v-alert
      v-if="hasOperation('videoCompression')"
      type="warning"
      variant="tonal"
      density="compact"
      class="mb-3"
      icon="mdi-alert-outline"
    >
      Static GIFs may be converted to images. That conversion always rebuilds all image metadata,
      the thumbnail, and visual hashes, even when those options are unchecked.
    </v-alert>

    <v-alert
      v-if="isDangerous"
      type="error"
      variant="tonal"
      density="compact"
      class="mb-3"
      icon="mdi-delete-alert-outline"
    >
      All tags on the {{ targetCount.toLocaleString() }} captured targets will be removed at commit
      time. This cannot be undone.
    </v-alert>

    <v-alert
      v-if="attemptedSubmit && selectedOperations.length === 0"
      type="warning"
      variant="tonal"
      density="compact"
      class="mb-3"
    >
      Select at least one operation before creating the job.
    </v-alert>

    <div class="d-flex justify-end mt-5">
      <v-btn
        type="submit"
        :color="isDangerous ? 'error' : 'primary'"
        :loading="submitting"
        :disabled="submitting"
        prepend-icon="mdi-play"
      >
        {{ isDangerous ? 'Create destructive job' : 'Create reindex job' }}
      </v-btn>
    </div>
  </v-form>
</template>

<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import type { ReindexOperation } from '@/api/reindex'
import { isDangerousReindexPlan, SAFE_REINDEX_OPERATIONS } from '@/api/reindex'
import ReindexOptionRow from './ReindexOptionRow.vue'

interface ReindexOptionDefinition {
  operation: ReindexOperation
  title: string
  description: string
  applicability: string
  danger?: boolean
}

interface ReindexOptionGroup {
  title: string
  options: ReindexOptionDefinition[]
}

defineProps<{
  targetCount: number
  submitting: boolean
}>()

const emit = defineEmits<{
  submit: [operations: ReindexOperation[]]
}>()

const optionGroups: ReindexOptionGroup[] = [
  {
    title: 'Metadata',
    options: [
      {
        operation: 'exif',
        title: 'EXIF and probe metadata',
        description: 'Read image EXIF or video stream metadata from the imported original.',
        applicability: 'Images & videos'
      },
      {
        operation: 'dimensions',
        title: 'Dimensions',
        description: 'Recalculate oriented width and height.',
        applicability: 'Images & videos'
      },
      {
        operation: 'fileSize',
        title: 'File size',
        description: 'Read the canonical imported file size again.',
        applicability: 'Images & videos'
      }
    ]
  },
  {
    title: 'Derived media',
    options: [
      {
        operation: 'thumbnail',
        title: 'Thumbnail',
        description: 'Regenerate the JPEG preview without changing visual hashes unless selected.',
        applicability: 'Images & videos'
      },
      {
        operation: 'visualHashes',
        title: 'Visual hashes',
        description: 'Recalculate thumbhash and image perceptual hash from the selected preview.',
        applicability: 'Images & videos'
      }
    ]
  },
  {
    title: 'Video',
    options: [
      {
        operation: 'videoCompression',
        title: 'Video compression',
        description: 'Rebuild the web MP4 only; it does not implicitly regenerate a thumbnail.',
        applicability: 'Videos only'
      }
    ]
  },
  {
    title: 'Danger',
    options: [
      {
        operation: 'clearTags',
        title: 'Clear all tags',
        description: 'Permanently remove every tag present on each target when it commits.',
        applicability: 'Images & videos',
        danger: true
      }
    ]
  }
]

const selectedOperations = shallowRef<ReindexOperation[]>([...SAFE_REINDEX_OPERATIONS])
const attemptedSubmit = shallowRef(false)
const isDangerous = computed(() => isDangerousReindexPlan(selectedOperations.value))

const hasOperation = (operation: ReindexOperation): boolean =>
  selectedOperations.value.includes(operation)

const setOperation = (operation: ReindexOperation, selected: boolean) => {
  const next = new Set(selectedOperations.value)
  if (selected) next.add(operation)
  else next.delete(operation)
  selectedOperations.value = [...next]
}

const submitPlan = () => {
  attemptedSubmit.value = true
  if (selectedOperations.value.length === 0) return
  emit('submit', [...selectedOperations.value])
}
</script>
