<script setup lang="ts">
import { computed, onUnmounted, shallowRef } from 'vue'
import { useDisplay } from 'vuetify'
import { useSearchHistoryStore } from '@/store/searchHistoryStore'
import AdvancedSearchDialog from './AdvancedSearchDialog.vue'
import MobileSearchDialog from './MobileSearchDialog.vue'
import SearchHistoryList from './SearchHistoryList.vue'
import type { GallerySortOrder } from '@/type/types'
import {
  getQuickSortPresentation,
  nextQuickSortOrder,
  type GallerySearchSubmission
} from '@/script/utils/gallerySort'

const props = withDefaults(
  defineProps<{
    sortOrder: GallerySortOrder
    halfWidth?: boolean
    canSave?: boolean
  }>(),
  {
    halfWidth: false,
    canSave: false
  }
)

const searchQuery = defineModel<string | null>({ required: true })

const emit = defineEmits<{
  search: [query: string]
  advancedSearch: [submission: GallerySearchSubmission]
  sort: [sortOrder: GallerySortOrder]
  save: [query: string]
}>()

const { smAndDown } = useDisplay()
const searchHistoryStore = useSearchHistoryStore()
const searchFocused = shallowRef(false)
const showMobileSearch = shallowRef(false)
const showAdvancedSearch = shallowRef(false)
let blurTimeout: ReturnType<typeof setTimeout> | null = null

const desktopCardStyle = computed(() => ({ width: props.halfWidth ? '50%' : '100%' }))
const quickSortPresentation = computed(() =>
  getQuickSortPresentation(props.sortOrder)
)
const saveDisabled = computed(
  () => !props.canSave || (searchQuery.value?.trim() ?? '') === ''
)

function clearBlurTimeout(): void {
  if (blurTimeout === null) return
  clearTimeout(blurTimeout)
  blurTimeout = null
}

function closeSearchHistory(): void {
  clearBlurTimeout()
  searchFocused.value = false
}

function onSearchFocus(): void {
  clearBlurTimeout()
  searchFocused.value = searchHistoryStore.history.length > 0
}

function onSearchBlur(): void {
  clearBlurTimeout()
  blurTimeout = setTimeout(() => {
    searchFocused.value = false
    blurTimeout = null
  }, 200)
}

function commitSearch(rawQuery: string | null): string {
  const query = rawQuery?.trim() ?? ''
  searchQuery.value = query === '' ? null : query

  if (query !== '') searchHistoryStore.add(query)

  closeSearchHistory()
  showMobileSearch.value = false
  return query
}

function applySearch(rawQuery: string | null): void {
  const query = commitSearch(rawQuery)
  emit('search', query)
}

function submitCurrentSearch(): void {
  applySearch(searchQuery.value)
}

function clearAndSearch(): void {
  applySearch(null)
}

function selectHistoryItem(query: string): void {
  applySearch(query)
}

function removeHistoryItem(index: number): void {
  searchHistoryStore.remove(index)
  if (searchHistoryStore.history.length === 0) closeSearchHistory()
}

function clearSearchHistory(): void {
  searchHistoryStore.clear()
  closeSearchHistory()
}

function openAdvancedSearch(): void {
  closeSearchHistory()
  showAdvancedSearch.value = true
}

function openAdvancedSearchFromMobile(): void {
  showMobileSearch.value = false
  showAdvancedSearch.value = true
}

function submitAdvancedSearch(submission: GallerySearchSubmission): void {
  const query = commitSearch(submission.query)
  emit('advancedSearch', { query, sortOrder: submission.sortOrder })
}

function changeSort(sortOrder: GallerySortOrder): void {
  showMobileSearch.value = false
  emit('sort', sortOrder)
}

function toggleSort(): void {
  changeSort(nextQuickSortOrder(props.sortOrder))
}

function saveCurrentSearch(): void {
  const query = searchQuery.value?.trim() ?? ''
  if (saveDisabled.value || query === '') return

  searchQuery.value = query
  closeSearchHistory()
  showMobileSearch.value = false
  emit('save', query)
}

onUnmounted(clearBlurTimeout)
</script>

<template>
  <v-card
    v-if="!smAndDown"
    elevation="0"
    class="search-control-card"
    :style="desktopCardStyle"
  >
    <v-card-text class="pa-0 bg-surface">
      <v-menu
        v-model="searchFocused"
        :close-on-content-click="false"
        :open-on-click="false"
        offset="4"
        max-height="400"
      >
        <template #activator="{ props: menuProps }">
          <v-text-field
            id="nav-search-input"
            v-model="searchQuery"
            rounded
            class="ma-0"
            bg-color="surface-light"
            label="Search"
            autocomplete="off"
            clearable
            persistent-clear
            variant="solo"
            flat
            prepend-inner-icon="mdi-magnify"
            single-line
            hide-details
            style="margin-right: 10px"
            v-bind="menuProps"
            @click:prepend-inner="submitCurrentSearch"
            @click:clear="clearAndSearch"
            @keyup.enter="submitCurrentSearch"
            @focus="onSearchFocus"
            @blur="onSearchBlur"
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
            <template #append-inner>
              <v-btn
                icon="mdi-bookmark-plus-outline"
                variant="text"
                size="small"
                aria-label="Save search"
                :disabled="saveDisabled"
                @mousedown.stop.prevent
                @click.stop="saveCurrentSearch"
              />
              <v-btn
                icon="mdi-tune"
                variant="text"
                size="small"
                aria-label="Advanced search"
                @mousedown.stop.prevent
                @click.stop="openAdvancedSearch"
              />
              <v-btn
                :icon="quickSortPresentation.icon"
                variant="text"
                size="small"
                :aria-label="quickSortPresentation.ariaLabel"
                @mousedown.stop.prevent
                @click.stop="toggleSort"
              />
            </template>
          </v-text-field>
        </template>

        <SearchHistoryList
          v-if="searchHistoryStore.history.length > 0"
          :history="searchHistoryStore.history"
          @select="selectHistoryItem"
          @remove="removeHistoryItem"
          @clear="clearSearchHistory"
        />
      </v-menu>
    </v-card-text>
  </v-card>

  <v-btn
    v-else
    variant="flat"
    color="surface-light"
    rounded
    height="48"
    class="flex-grow-1 mx-2"
    aria-label="Search"
    @click="showMobileSearch = true"
  >
    <v-icon size="24">mdi-magnify</v-icon>
  </v-btn>

  <AdvancedSearchDialog
    v-if="showAdvancedSearch"
    v-model="showAdvancedSearch"
    :sort-order="props.sortOrder"
    @search="submitAdvancedSearch"
  />

  <MobileSearchDialog
    v-if="showMobileSearch"
    v-model="showMobileSearch"
    v-model:search-query="searchQuery"
    :history="searchHistoryStore.history"
    :sort-order="props.sortOrder"
    :can-save="props.canSave"
    @search="applySearch"
    @save="saveCurrentSearch"
    @select-history="selectHistoryItem"
    @remove-history="removeHistoryItem"
    @clear-history="clearSearchHistory"
    @open-advanced-search="openAdvancedSearchFromMobile"
    @sort="changeSort"
  />
</template>

<style scoped>
.search-control-card {
  flex: 1 1 auto;
  min-width: 0;
}

.search-control-card :deep(.v-field),
.search-control-card :deep(.v-field input) {
  cursor: text;
}

.search-control-card :deep(.v-btn) {
  cursor: pointer;
}
</style>
