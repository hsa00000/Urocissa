<script setup lang="ts">
import SearchHistoryList from './SearchHistoryList.vue'

const isOpen = defineModel<boolean>({ required: true })
const searchQuery = defineModel<string | null>('searchQuery', { required: true })

defineProps<{
  history: readonly string[]
}>()

const emit = defineEmits<{
  search: [query: string | null]
  selectHistory: [query: string]
  removeHistory: [index: number]
  clearHistory: []
  openAdvancedSearch: []
}>()

function submitSearch(): void {
  emit('search', searchQuery.value)
}

function clearAndSearch(): void {
  searchQuery.value = null
  emit('search', null)
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
          prepend-inner-icon="mdi-magnify"
          single-line
          hide-details
          class="mr-2"
          @click:prepend-inner="submitSearch"
          @click:clear="clearAndSearch"
          @keyup.enter="submitSearch"
        >
          <template #label>
            <span class="text-caption">Search</span>
          </template>
        </v-text-field>

        <v-btn
          icon="mdi-tune"
          aria-label="Advanced search"
          @click="emit('openAdvancedSearch')"
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
