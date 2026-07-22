<script setup lang="ts">
import { computed, reactive } from 'vue'
import { useDisplay } from 'vuetify'
import { useTagStore } from '@/store/tagStore'
import {
  buildAdvancedSearchFilter,
  createEmptyAdvancedSearchCriteria,
  type AdvancedSearchCriteria
} from './advancedSearch'

const isOpen = defineModel<boolean>({ required: true })

const emit = defineEmits<{
  search: [filterString: string]
}>()

const { smAndDown } = useDisplay()
const tagStore = useTagStore('mainId')
const criteria = reactive<AdvancedSearchCriteria>(createEmptyAdvancedSearchCriteria())
const tagItems = computed(() => tagStore.tags.map((tagInfo) => tagInfo.tag))

function clearAll(): void {
  Object.assign(criteria, createEmptyAdvancedSearchCriteria())
}

function submitSearch(): void {
  emit('search', buildAdvancedSearchFilter(criteria))
  isOpen.value = false
}
</script>

<template>
  <v-dialog v-model="isOpen" :fullscreen="smAndDown" max-width="600">
    <v-card class="d-flex flex-column">
      <v-card-title class="d-flex align-center">
        <v-icon class="mr-2">mdi-tune</v-icon>
        <span class="text-h6">Search Options</span>
        <v-spacer />
        <v-btn icon="mdi-close" variant="text" aria-label="Close" @click="isOpen = false" />
      </v-card-title>

      <v-card-text class="flex-grow-1 overflow-y-auto">
        <v-text-field
          v-model="criteria.keyword"
          label="Keyword"
          variant="outlined"
          density="comfortable"
          clearable
          hide-details
          autocomplete="off"
          class="mb-4"
        />

        <v-text-field
          v-model="criteria.filename"
          label="Filename"
          variant="outlined"
          density="comfortable"
          clearable
          hide-details
          autocomplete="off"
          class="mb-4"
        />

        <v-combobox
          v-model="criteria.tag"
          :items="tagItems"
          :hide-no-data="false"
          label="Tag"
          menu-icon="mdi-menu-down"
          no-data-text="No tags available"
          variant="outlined"
          density="comfortable"
          clearable
          hide-details
          autocomplete="off"
          class="mb-4"
        />

        <v-text-field
          v-model="criteria.extension"
          label="File Extension"
          placeholder="e.g. jpg, png, mp4"
          variant="outlined"
          density="comfortable"
          clearable
          hide-details
          autocomplete="off"
          class="mb-4"
        />

        <div class="text-subtitle-2 mb-2">Camera</div>
        <v-row density="comfortable" class="mb-4">
          <v-col cols="6">
            <v-text-field
              v-model="criteria.cameraMake"
              label="Make"
              variant="outlined"
              density="comfortable"
              clearable
              hide-details
              autocomplete="off"
            />
          </v-col>
          <v-col cols="6">
            <v-text-field
              v-model="criteria.cameraModel"
              label="Model"
              variant="outlined"
              density="comfortable"
              clearable
              hide-details
              autocomplete="off"
            />
          </v-col>
        </v-row>

        <div class="text-subtitle-2 mb-2">Media Type</div>
        <v-btn-toggle
          v-model="criteria.mediaType"
          mandatory
          color="primary"
          border
          divided
          class="d-flex"
        >
          <v-btn value="all" class="flex-grow-1">All</v-btn>
          <v-btn value="image" class="flex-grow-1">Image</v-btn>
          <v-btn value="video" class="flex-grow-1">Video</v-btn>
          <v-btn value="album" class="flex-grow-1">Album</v-btn>
        </v-btn-toggle>
      </v-card-text>

      <v-card-actions>
        <v-btn variant="text" @click="clearAll">Clear All</v-btn>
        <v-spacer />
        <v-btn color="primary" variant="elevated" @click="submitSearch">Search</v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>
