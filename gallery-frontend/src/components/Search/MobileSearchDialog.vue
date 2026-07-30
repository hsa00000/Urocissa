<script setup lang="ts">
import { computed } from 'vue'
import SearchHistoryList from './SearchHistoryList.vue'
import type { GallerySortOrder } from '@/type/types'
import {
  getQuickSortPresentation,
  nextQuickSortOrder
} from '@/script/utils/gallerySort'

const isOpen = defineModel<boolean>({ required: true })
const searchQuery = defineModel<string | null>('searchQuery', { required: true })

const props = withDefaults(
  defineProps<{
    history: readonly string[]
    sortOrder: GallerySortOrder
    canSave?: boolean
    showSaveAction?: boolean
  }>(),
  {
    canSave: false,
    showSaveAction: true
  }
)

const emit = defineEmits<{
  search: [query: string | null]
  save: []
  selectHistory: [query: string]
  removeHistory: [index: number]
  clearHistory: []
  openAdvancedSearch: []
  sort: [sortOrder: GallerySortOrder]
}>()

const quickSortPresentation = computed(() =>
  getQuickSortPresentation(props.sortOrder)
)

function submitSearch(): void {
  emit('search', searchQuery.value)
}

function clearAndSearch(): void {
  searchQuery.value = null
  emit('search', null)
}

function toggleSort(): void {
  emit('sort', nextQuickSortOrder(props.sortOrder))
}
</script>

<template>
  <v-dialog v-model="isOpen" fullscreen transition="dialog-bottom-transition">
    <v-card class="d-flex flex-column">
      <v-toolbar class="bg-surface py-2" density="comfortable">
        <v-btn icon="mdi-arrow-left" aria-label="Close search" @click="isOpen = false" />

        <v-text-field
          v-model="searchQuery"
          bg-color="surface-light"
          autocomplete="off"
          autofocus
          clearable
          persistent-clear
          variant="solo"
          flat
          rounded
          label="Search"
          prepend-inner-icon="mdi-magnify"
          single-line
          hide-details
          class="mr-2"
          @click:prepend-inner="submitSearch"
          @click:clear="clearAndSearch"
          @keyup.enter="submitSearch"
        >
          <template #clear="{ props: clearProps }">
            <v-btn
              v-bind="clearProps"
              icon="mdi-close-circle"
              variant="text"
              size="small"
              tabindex="0"
              aria-label="Clear search"
            />
          </template>
        </v-text-field>

        <v-btn
          v-if="props.showSaveAction"
          icon="mdi-bookmark-plus-outline"
          variant="text"
          size="small"
          aria-label="Save search"
          :disabled="!props.canSave || (searchQuery?.trim() ?? '') === ''"
          @click="emit('save')"
        />
        <v-btn
          icon="mdi-tune"
          variant="text"
          size="small"
          aria-label="Advanced search"
          @click="emit('openAdvancedSearch')"
        />
        <v-btn
          :icon="quickSortPresentation.icon"
          variant="text"
          size="small"
          :aria-label="quickSortPresentation.ariaLabel"
          @click="toggleSort"
        />
      </v-toolbar>

      <SearchHistoryList
        class="flex-grow-1 overflow-y-auto"
        :history="history"
        show-empty
        @select="emit('selectHistory', $event)"
        @remove="emit('removeHistory', $event)"
        @clear="emit('clearHistory')"
      />
    </v-card>
  </v-dialog>
</template>
