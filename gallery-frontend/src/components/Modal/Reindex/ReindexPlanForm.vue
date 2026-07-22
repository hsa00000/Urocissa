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
  danger?: boolean
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
    danger: true,
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

<template>
  <v-form @submit.prevent="submitPlan">
    <v-list bg-color="transparent" class="pa-0 mb-4">
      <v-list-item
        title="Choose what to rebuild"
        :subtitle="`${targetCount.toLocaleString()} selected ${targetCount === 1 ? 'object' : 'objects'}`"
        lines="two"
        class="px-0"
      >
        <template #prepend>
          <v-avatar color="primary" variant="tonal" size="42">
            <v-icon icon="mdi-image-sync-outline" />
          </v-avatar>
        </template>
      </v-list-item>
    </v-list>

    <v-list
      v-for="group in optionGroups"
      :key="group.title"
      border
      rounded="lg"
      bg-color="transparent"
      :lines="false"
      class="pa-0 mb-4"
    >
      <v-list-subheader
        :title="group.title"
        :color="group.danger === true ? 'error' : undefined"
      />
      <v-divider />

      <template v-for="(option, index) in group.options" :key="option.operation">
        <ReindexOptionRow
          :model-value="hasOperation(option.operation)"
          :title="option.title"
          :description="option.description"
          :applicability="option.applicability"
          :danger="option.danger"
          @update:model-value="setOperation(option.operation, $event)"
        />
        <v-divider v-if="index < group.options.length - 1" />
      </template>
    </v-list>

    <v-alert
      v-if="hasOperation('videoCompression')"
      type="warning"
      variant="tonal"
      density="compact"
      class="mb-3"
      icon="mdi-alert-outline"
      text="Static GIFs may be converted to images. That conversion always rebuilds all image metadata, the thumbnail, and visual hashes, even when those options are unchecked."
    />

    <v-alert
      v-if="isDangerous"
      type="error"
      variant="tonal"
      density="compact"
      class="mb-3"
      icon="mdi-delete-alert-outline"
      :text="`All tags on the ${targetCount.toLocaleString()} captured targets will be removed at commit time. This cannot be undone.`"
    />

    <v-alert
      v-if="attemptedSubmit && selectedOperations.length === 0"
      type="warning"
      variant="tonal"
      density="compact"
      class="mb-3"
      text="Select at least one operation before creating the job."
    />

    <v-card-actions class="pa-0 pt-2">
      <v-spacer />
      <v-btn
        type="submit"
        :color="isDangerous ? 'error' : 'primary'"
        :loading="submitting"
        :disabled="submitting"
        prepend-icon="mdi-play"
        :text="isDangerous ? 'Create destructive job' : 'Create reindex job'"
      />
    </v-card-actions>
  </v-form>
</template>
