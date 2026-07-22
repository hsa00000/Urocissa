<script setup lang="ts">
import type { AlbumInfo } from '@type/types'

defineProps<{
  albums: readonly AlbumInfo[]
  tagSuggestions: readonly string[]
  hasActiveWork: boolean
}>()

const presignedAlbumIds = defineModel<string[]>('presignedAlbumIds', { required: true })
const presignedTags = defineModel<string[]>('presignedTags', { required: true })

const emit = defineEmits<{
  addFiles: []
  cancelAll: []
  showFormats: []
}>()
</script>

<template>
  <div class="upload-controls border-b bg-surface">
    <div class="upload-presets px-4 pt-3 pb-2">
      <div class="text-body-2 font-weight-medium text-medium-emphasis mb-2">Presets</div>

      <v-row class="ma-0" density="comfortable">
        <v-col cols="12" sm="6" class="pa-0 pr-sm-2 pb-2">
          <v-autocomplete
            v-model="presignedAlbumIds"
            :items="albums"
            item-title="displayName"
            item-value="albumId"
            label="Presigned albums"
            density="compact"
            variant="outlined"
            clearable
            multiple
            chips
            closable-chips
            :hide-no-data="false"
            menu-icon="mdi-menu-down"
            no-data-text="No albums available"
            hide-details
            autocomplete="off"
          />
        </v-col>

        <v-col cols="12" sm="6" class="pa-0 pl-sm-2 pb-2">
          <v-combobox
            v-model="presignedTags"
            :items="tagSuggestions"
            label="Presigned tags"
            density="compact"
            variant="outlined"
            clearable
            multiple
            chips
            closable-chips
            :hide-no-data="false"
            menu-icon="mdi-menu-down"
            no-data-text="No tags available"
            hide-details
            autocomplete="off"
          />
        </v-col>
      </v-row>

    </div>

    <div class="d-flex align-center flex-wrap ga-2 px-4 py-2">
      <v-btn
        color="primary"
        variant="flat"
        prepend-icon="mdi-plus"
        class="text-none"
        data-testid="upload-add-files"
        @click="emit('addFiles')"
      >
        Add Files
      </v-btn>

      <v-btn
        variant="text"
        prepend-icon="mdi-cancel"
        class="text-none text-medium-emphasis"
        :disabled="!hasActiveWork"
        @click="emit('cancelAll')"
      >
        Cancel All
      </v-btn>

      <v-spacer />

      <v-btn
        variant="text"
        prepend-icon="mdi-file-question-outline"
        class="text-none"
        @click="emit('showFormats')"
      >
        Supported Formats
      </v-btn>
    </div>
  </div>
</template>

<style scoped>
.upload-controls {
  flex: 0 0 auto;
}

.upload-presets {
  max-width: 900px;
}
</style>
