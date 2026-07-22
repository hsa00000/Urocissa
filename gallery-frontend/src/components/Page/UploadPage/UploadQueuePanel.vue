<script setup lang="ts">
import { computed, shallowRef } from 'vue'
import type { AlbumInfo } from '@type/types'
import {
  useUploadStore,
  type UploadPresentationOptions,
  type UploadTarget
} from '@/store/uploadStore'
import UploadControls from './UploadControls.vue'
import UploadQueueTable from './UploadQueueTable.vue'

const emit = defineEmits<{
  showFormats: []
}>()

const presignedAlbumIds = defineModel<string[]>('presignedAlbumIds', { required: true })
const presignedTags = defineModel<string[]>('presignedTags', { required: true })
const props = defineProps<{
  albums: readonly AlbumInfo[]
  tagSuggestions: readonly string[]
}>()

const uploadStore = useUploadStore('mainId')
const currentTab = shallowRef<'uploading' | 'success' | 'issues'>('uploading')
const silentSummary: UploadPresentationOptions = { showSummaryPanel: false }

const tabs = computed(() => [
  {
    value: 'uploading' as const,
    label: 'Uploading',
    icon: 'mdi-upload',
    count: uploadStore.uploadingItems.length
  },
  {
    value: 'success' as const,
    label: 'Success',
    icon: 'mdi-check-circle',
    count: uploadStore.successItems.length
  },
  {
    value: 'issues' as const,
    label: 'Errors',
    icon: 'mdi-alert-circle',
    count: uploadStore.issueItems.length
  }
])

const selectedTarget = computed<UploadTarget | undefined>(() => {
  const albums = presignedAlbumIds.value.flatMap((albumId) => {
    const album = props.albums.find((candidate) => candidate.albumId === albumId)
    return album === undefined ? [] : [{ id: album.albumId, name: album.displayName }]
  })
  const tags = [
    ...new Set(presignedTags.value.map((tag) => tag.trim()).filter((tag) => tag !== ''))
  ]
  if (albums.length === 0 && tags.length === 0) return undefined
  return { albums, tags }
})

function addFiles(): void {
  uploadStore.openFilePicker(selectedTarget.value, silentSummary)
}

function retryItem(itemId: string): void {
  uploadStore.retryItem(itemId, silentSummary)
}

function retryAll(): void {
  uploadStore.retryAll(silentSummary)
}
</script>

<template>
  <div class="upload-queue-panel d-flex flex-column h-100">
    <UploadControls
      v-model:presigned-album-ids="presignedAlbumIds"
      v-model:presigned-tags="presignedTags"
      :albums="albums"
      :tag-suggestions="tagSuggestions"
      :has-active-work="uploadStore.hasActiveWork"
      @add-files="addFiles"
      @cancel-all="uploadStore.cancelAll"
      @show-formats="emit('showFormats')"
    />

    <v-tabs
      v-model="currentTab"
      color="primary"
      density="comfortable"
      show-arrows
      class="upload-tabs border-b flex-grow-0"
    >
      <v-tab
        v-for="tab in tabs"
        :key="tab.value"
        :value="tab.value"
        class="text-none"
      >
        <v-icon :icon="tab.icon" />
        <span class="d-none d-sm-inline ml-2">{{ tab.label }}</span>
        <v-chip size="x-small" class="ml-2" variant="tonal">{{ tab.count }}</v-chip>
      </v-tab>
    </v-tabs>

    <v-tabs-window v-model="currentTab" class="upload-tab-window flex-grow-1 bg-surface">
      <v-tabs-window-item value="uploading" class="upload-tab-item">
        <div class="upload-tab-pane">
          <div v-if="uploadStore.uploadingItems.length === 0" class="upload-empty-state">
            <v-icon icon="mdi-cloud-upload-outline" size="72" color="medium-emphasis" />
            <div class="text-h6 mt-3">Drop photos and videos anywhere</div>
            <div class="text-body-2 text-medium-emphasis mt-1 mb-4">
              Or choose files to start uploading immediately.
            </div>
            <v-btn color="primary" prepend-icon="mdi-plus" @click="addFiles">Choose Files</v-btn>
          </div>
          <UploadQueueTable
            v-else
            :items="uploadStore.uploadingItems"
            mode="uploading"
            @cancel="uploadStore.cancelItem"
            @retry="retryItem"
            @clear="uploadStore.clearItem"
          />
        </div>
      </v-tabs-window-item>

      <v-tabs-window-item value="success" class="upload-tab-item">
        <div class="upload-tab-pane">
          <div class="d-flex align-center justify-end px-4 py-2 border-b flex-grow-0">
            <v-btn
              variant="text"
              prepend-icon="mdi-notification-clear-all"
              :disabled="uploadStore.successItems.length === 0"
              @click="uploadStore.clearCompleted"
            >
              Clear Completed
            </v-btn>
          </div>
          <div v-if="uploadStore.successItems.length === 0" class="upload-empty-state">
            <v-icon icon="mdi-check-circle-outline" size="64" color="medium-emphasis" />
            <div class="text-body-1 text-medium-emphasis mt-3">
              Completed uploads appear here.
            </div>
          </div>
          <UploadQueueTable
            v-else
            :items="uploadStore.successItems"
            mode="success"
            @cancel="uploadStore.cancelItem"
            @retry="retryItem"
            @clear="uploadStore.clearItem"
          />
        </div>
      </v-tabs-window-item>

      <v-tabs-window-item value="issues" class="upload-tab-item">
        <div class="upload-tab-pane">
          <div class="d-flex align-center flex-wrap ga-2 px-4 py-2 border-b flex-grow-0">
            <v-btn
              variant="text"
              color="primary"
              prepend-icon="mdi-refresh"
              :disabled="uploadStore.issueItems.length === 0"
              @click="retryAll"
            >
              Retry All
            </v-btn>
            <v-spacer />
            <v-btn
              variant="text"
              prepend-icon="mdi-notification-clear-all"
              :disabled="uploadStore.issueItems.length === 0"
              @click="uploadStore.clearIssues"
            >
              Clear Errors
            </v-btn>
          </div>
          <div v-if="uploadStore.issueItems.length === 0" class="upload-empty-state">
            <v-icon icon="mdi-check-decagram-outline" size="64" color="success" />
            <div class="text-body-1 text-medium-emphasis mt-3">No upload errors.</div>
          </div>
          <UploadQueueTable
            v-else
            :items="uploadStore.issueItems"
            mode="issues"
            @cancel="uploadStore.cancelItem"
            @retry="retryItem"
            @clear="uploadStore.clearItem"
          />
        </div>
      </v-tabs-window-item>
    </v-tabs-window>
  </div>
</template>

<style scoped>
.upload-queue-panel {
  min-height: 0;
}

.upload-tabs {
  flex: 0 0 auto;
  min-height: 48px;
}

.upload-tab-window {
  flex: 1 1 0;
  min-height: 0;
  overflow: hidden;
}

.upload-tab-window :deep(.v-window__container),
.upload-tab-window :deep(.v-window-item) {
  height: 100%;
  min-height: 0;
}

.upload-tab-pane {
  display: flex;
  flex-direction: column;
  height: 100%;
  min-height: 0;
}

.upload-empty-state {
  flex: 1 1 auto;
  min-height: 0;
  overflow-y: auto;
  padding: 48px 24px;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
}
</style>
