<script setup lang="ts">
import { computed, reactive } from 'vue'
import { useDisplay } from 'vuetify'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import {
  buildAdvancedSearchFilter,
  createEmptyAdvancedSearchCriteria,
  type AdvancedSearchCriteria
} from './advancedSearch'
import type { GallerySortOrder } from '@/type/types'
import type { GallerySearchSubmission } from '@/script/utils/gallerySort'

const isOpen = defineModel<boolean>({ required: true })

const props = withDefaults(
  defineProps<{
    sortOrder: GallerySortOrder
    includeAlbumMediaType?: boolean
  }>(),
  {
    includeAlbumMediaType: true
  }
)

const emit = defineEmits<{
  search: [submission: GallerySearchSubmission]
}>()

const { smAndDown } = useDisplay()
const searchFacetStore = useSearchFacetStore()
const criteria = reactive<AdvancedSearchCriteria>(
  createEmptyAdvancedSearchCriteria(props.sortOrder)
)
const tagItems = computed(() => searchFacetStore.tags.map((facet) => facet.value))
const makeItems = computed(() => searchFacetStore.makes.map((facet) => facet.value))
const modelItems = computed(() => searchFacetStore.models.map((facet) => facet.value))

function clearAll(): void {
  Object.assign(criteria, createEmptyAdvancedSearchCriteria())
}

function submitSearch(): void {
  emit('search', {
    query: buildAdvancedSearchFilter(criteria),
    sortOrder: criteria.sortOrder
  })
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

        <v-label text="Camera" class="text-subtitle-2 mb-2" />
        <v-row density="comfortable" class="mb-4" aria-label="Camera">
          <v-col cols="6">
            <v-combobox
              v-model="criteria.cameraMake"
              :items="makeItems"
              :hide-no-data="false"
              label="Make"
              menu-icon="mdi-menu-down"
              no-data-text="No camera makes available"
              variant="outlined"
              density="comfortable"
              clearable
              hide-details
              autocomplete="off"
            />
          </v-col>
          <v-col cols="6">
            <v-combobox
              v-model="criteria.cameraModel"
              :items="modelItems"
              :hide-no-data="false"
              label="Model"
              menu-icon="mdi-menu-down"
              no-data-text="No camera models available"
              variant="outlined"
              density="comfortable"
              clearable
              hide-details
              autocomplete="off"
            />
          </v-col>
        </v-row>

        <v-label
          for="advanced-media-type-toggle"
          text="Media Type"
          class="text-subtitle-2 mb-2"
        />
        <v-btn-toggle
          id="advanced-media-type-toggle"
          v-model="criteria.mediaType"
          mandatory
          color="primary"
          border
          divided
          class="d-flex"
          aria-label="Media type"
        >
          <v-btn value="all" class="flex-grow-1">All</v-btn>
          <v-btn value="image" class="flex-grow-1">Image</v-btn>
          <v-btn value="video" class="flex-grow-1">Video</v-btn>
          <v-btn v-if="props.includeAlbumMediaType" value="album" class="flex-grow-1">
            Album
          </v-btn>
        </v-btn-toggle>

        <v-label
          for="advanced-sort-order-toggle"
          text="Sort Order"
          class="text-subtitle-2 mt-4 mb-2"
        />
        <v-btn-toggle
          id="advanced-sort-order-toggle"
          v-model="criteria.sortOrder"
          mandatory
          color="primary"
          border
          divided
          class="d-flex"
          aria-label="Sort order"
        >
          <v-btn value="descending" class="flex-grow-1">Descending</v-btn>
          <v-btn value="ascending" class="flex-grow-1">Ascending</v-btn>
          <v-btn value="random" class="flex-grow-1">Random</v-btn>
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
