<script setup lang="ts">
import { computed } from 'vue'
import { filesize } from 'filesize'
import { useDisplay } from 'vuetify'
import type { UploadQueueItem } from '@/store/uploadStore'
import UploadStatus from './UploadStatus.vue'

type TableMode = 'uploading' | 'success' | 'issues'

interface PresetChip {
  key: string
  label: string
  icon: string
  color?: string
}

const props = defineProps<{
  items: readonly UploadQueueItem[]
  mode: TableMode
}>()

const emit = defineEmits<{
  cancel: [itemId: string]
  retry: [itemId: string]
  clear: [itemId: string]
}>()

const { mobile } = useDisplay()
const visiblePresetCount = 3

const headers = computed(() => {
  const base = [
    { title: '#', key: 'rowNumber', sortable: false, width: '64px' },
    { title: 'File name', key: 'fileName' },
    { title: 'Size', key: 'size', width: '110px' },
    { title: 'Presets', key: 'presets', sortable: false, width: '280px' },
    { title: 'Status', key: 'uploadStatus', sortable: false, width: '190px' }
  ]

  if (props.mode === 'success') {
    base.push({ title: 'Time', key: 'elapsed', sortable: false, width: '110px' })
  } else if (props.mode === 'issues') {
    base.push({ title: 'Reason', key: 'reason', sortable: false, width: '220px' })
  }
  base.push({ title: '', key: 'actions', sortable: false, width: '120px' })
  return base
})

const tableItems = computed(() =>
  props.items.map((item, index) => {
    const allPresetChips = getPresetChips(item)
    return {
      ...item,
      rowNumber: index + 1,
      fileName: item.file.name,
      size: filesize(item.file.size),
      allPresetChips,
      presetChips: allPresetChips.slice(0, visiblePresetCount),
      hiddenPresetCount: Math.max(0, allPresetChips.length - visiblePresetCount),
      presetText: allPresetChips.map((preset) => preset.label).join(', '),
      elapsed: formatElapsed(item),
      reason: item.errorReason ?? '—'
    }
  })
)

function getPresetChips(item: UploadQueueItem): PresetChip[] {
  const albums = (item.target.albums ?? []).map((album) => ({
    key: `album-${album.id}`,
    label: album.name ?? album.id,
    icon: 'mdi-image-album'
  }))
  const tags = (item.target.tags ?? []).map((tag) => ({
    key: `tag-${tag}`,
    label: tag,
    icon: 'mdi-tag-outline',
    color: 'secondary'
  }))
  return [...albums, ...tags]
}

function formatElapsed(item: UploadQueueItem): string {
  if (item.startedAt === undefined || item.endedAt === undefined) return '—'
  const seconds = Math.max(0, Math.round((item.endedAt - item.startedAt) / 1000))
  if (seconds < 60) return `${seconds}s`
  return `${Math.floor(seconds / 60)}m ${seconds % 60}s`
}
</script>

<template>
  <div class="upload-queue-table">
    <div v-if="mobile" class="upload-mobile-list pa-3">
      <v-card
        v-for="item in tableItems"
        :key="item.id"
        variant="tonal"
        class="upload-mobile-card mb-3"
      >
        <v-card-item>
          <template #prepend>
            <v-avatar color="surface" size="36">{{ item.rowNumber }}</v-avatar>
          </template>
          <v-card-title class="text-body-1 text-truncate">{{ item.fileName }}</v-card-title>
          <v-card-subtitle>{{ item.size }}</v-card-subtitle>
        </v-card-item>

        <v-card-text class="pt-1">
          <div v-if="item.allPresetChips.length > 0" class="d-flex flex-wrap ga-1 mb-3">
            <v-chip
              v-for="preset in item.allPresetChips"
              :key="preset.key"
              :prepend-icon="preset.icon"
              :color="preset.color"
              size="x-small"
              variant="tonal"
            >
              {{ preset.label }}
            </v-chip>
          </div>
          <UploadStatus :status="item.status" :loaded="item.loaded" :total="item.total" />
          <div v-if="mode === 'success'" class="text-caption text-medium-emphasis mt-3">
            Completed in {{ item.elapsed }}
          </div>
          <div v-if="mode === 'issues'" class="text-caption text-error mt-3">
            {{ item.errorReason ?? 'Upload canceled' }}
          </div>
        </v-card-text>

        <v-card-actions class="upload-mobile-actions justify-end">
          <v-btn
            v-if="mode === 'uploading'"
            variant="text"
            color="warning"
            prepend-icon="mdi-close"
            @click="emit('cancel', item.id)"
          >
            Cancel
          </v-btn>
          <v-btn
            v-if="mode === 'issues'"
            variant="text"
            color="primary"
            prepend-icon="mdi-refresh"
            @click="emit('retry', item.id)"
          >
            Retry
          </v-btn>
          <v-btn
            v-if="mode !== 'uploading'"
            variant="text"
            prepend-icon="mdi-close"
            @click="emit('clear', item.id)"
          >
            Clear
          </v-btn>
        </v-card-actions>
      </v-card>
    </div>

    <v-data-table
      v-else
      :headers="headers"
      :items="tableItems"
      item-value="id"
      fixed-header
      hover
      height="100%"
      :items-per-page="25"
      class="upload-data-table bg-surface h-100"
    >
      <template #[`item.fileName`]="{ item }">
        <span class="d-block text-truncate upload-file-name">{{ item.fileName }}</span>
      </template>

      <template #[`item.presets`]="{ item }">
        <div
          v-if="item.presetChips.length > 0"
          class="d-flex align-center ga-1"
          :title="item.presetText"
        >
          <v-chip
            v-for="preset in item.presetChips"
            :key="preset.key"
            :prepend-icon="preset.icon"
            :color="preset.color"
            size="x-small"
            variant="tonal"
          >
            <span class="upload-preset-label text-truncate">{{ preset.label }}</span>
          </v-chip>
          <v-chip v-if="item.hiddenPresetCount > 0" size="x-small" variant="outlined">
            +{{ item.hiddenPresetCount }}
          </v-chip>
        </div>
        <span v-else>—</span>
      </template>

      <template #[`item.uploadStatus`]="{ item }">
        <UploadStatus :status="item.status" :loaded="item.loaded" :total="item.total" />
      </template>

      <template #[`item.reason`]="{ item }">
        <span class="text-caption text-error">{{ item.reason }}</span>
      </template>

      <template #[`item.actions`]="{ item }">
        <div class="upload-row-actions">
          <v-btn
            v-if="mode === 'uploading'"
            icon="mdi-close"
            variant="text"
            color="warning"
            aria-label="Cancel upload"
            @click="emit('cancel', item.id)"
          />
          <template v-else-if="mode === 'issues'">
            <v-btn
              icon="mdi-refresh"
              variant="text"
              color="primary"
              aria-label="Retry upload"
              @click="emit('retry', item.id)"
            />
            <v-btn
              icon="mdi-close"
              variant="text"
              aria-label="Clear upload"
              @click="emit('clear', item.id)"
            />
          </template>
          <v-btn
            v-else
            icon="mdi-close"
            variant="text"
            aria-label="Clear upload"
            @click="emit('clear', item.id)"
          />
        </div>
      </template>
    </v-data-table>
  </div>
</template>

<style scoped>
.upload-queue-table {
  flex: 1 1 0;
  min-height: 0;
  height: 100%;
  overflow: hidden;
}

.upload-mobile-list {
  height: 100%;
  min-height: 0;
  overflow-y: auto;
}

.upload-mobile-card:last-child {
  margin-bottom: 0 !important;
}

.upload-mobile-actions,
.upload-row-actions {
  display: flex;
  flex-flow: row nowrap;
  align-items: center;
}

.upload-row-actions {
  justify-content: center;
  min-width: 96px;
}

.upload-file-name {
  max-width: 360px;
}

.upload-preset-label {
  display: inline-block;
  max-width: 120px;
}

.upload-data-table :deep(.v-data-table__td) {
  max-width: 360px;
}

.upload-data-table :deep(th:last-child),
.upload-data-table :deep(td:last-child) {
  width: 120px;
  min-width: 120px;
}

.upload-data-table :deep(.v-table__wrapper) {
  overflow-y: auto;
}
</style>
