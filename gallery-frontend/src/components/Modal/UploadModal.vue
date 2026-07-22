<script setup lang="ts">
import { computed, onMounted, onUnmounted } from 'vue'
import { useRouter } from 'vue-router'
import humanizeDuration from 'humanize-duration'
import { handlePriorityEscape } from '@/script/utils/priorityEscape'
import { useModalStore } from '@/store/modalStore'
import { useUploadStore } from '@/store/uploadStore'

const router = useRouter()
const uploadStore = useUploadStore('mainId')
const modalStore = useModalStore('mainId')

const title = computed(() => {
  if (uploadStore.hasActiveWork) {
    return `Uploading ${uploadStore.currentRunCompletedCount + 1} of ${uploadStore.currentRunTotalCount}`
  }
  if (uploadStore.currentRunErrorCount > 0) return 'Upload finished with errors'
  return 'Upload complete'
})

const subtitle = computed(() => {
  const currentName = uploadStore.currentItem?.file.name
  if (currentName !== undefined) return currentName
  return `${uploadStore.currentRunSuccessCount} succeeded · ${uploadStore.currentRunErrorCount} issues`
})

const remainingLabel = computed(() => {
  if (!uploadStore.hasActiveWork || uploadStore.remainingTime <= 0) return undefined
  return `${humanizeDuration(uploadStore.remainingTime * 1000, {
    units: ['h', 'm', 's'],
    largest: 1,
    round: true
  })} remaining`
})

const statusIcon = computed(() => {
  if (uploadStore.hasActiveWork) return 'mdi-cloud-upload'
  if (uploadStore.currentRunErrorCount > 0) return 'mdi-alert-circle'
  return 'mdi-cloud-check-variant'
})

const statusColor = computed(() => {
  if (uploadStore.currentRunErrorCount > 0) return 'error'
  return uploadStore.hasActiveWork ? 'primary' : 'success'
})

function showDetails(): void {
  void router.push({ name: 'upload' })
}

function closePanel(): void {
  modalStore.showUploadModal = false
}

function handleKeydown(event: KeyboardEvent): void {
  handlePriorityEscape(event, closePanel)
}

onMounted(() => {
  window.addEventListener('keydown', handleKeydown, true)
})

onUnmounted(() => {
  window.removeEventListener('keydown', handleKeydown, true)
})
</script>

<template>
  <v-card
    id="upload-vcard"
    class="upload-summary-card mx-auto position-fixed"
    :title="title"
    :subtitle="subtitle"
    variant="elevated"
  >
    <template #prepend>
      <v-progress-circular
        :model-value="uploadStore.currentRunProgressPercent"
        :color="statusColor"
        size="48"
        width="4"
        class="ma-4"
      >
        <v-icon :icon="statusIcon" />
      </v-progress-circular>
    </template>

    <v-card-text v-if="remainingLabel" class="pt-0 text-caption text-medium-emphasis">
      {{ remainingLabel }}
    </v-card-text>

    <v-card-actions class="justify-end px-4 pb-4">
      <v-btn variant="text" prepend-icon="mdi-format-list-bulleted" @click="showDetails">
        Details
      </v-btn>
      <v-btn
        v-if="uploadStore.hasActiveWork"
        variant="outlined"
        color="warning"
        @click="uploadStore.cancelAll"
      >
        Cancel All
      </v-btn>
      <v-btn v-else variant="outlined" @click="closePanel">
        Close
      </v-btn>
    </v-card-actions>
  </v-card>
</template>

<style scoped>
.upload-summary-card {
  left: 24px;
  bottom: 24px;
  z-index: 50000;
  width: min(420px, calc(100vw - 48px));
}
</style>
