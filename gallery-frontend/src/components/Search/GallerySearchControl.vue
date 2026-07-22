<script setup lang="ts">
import { computed, onUnmounted, shallowRef } from 'vue'
import { useDisplay } from 'vuetify'
import { useSearchHistoryStore } from '@/store/searchHistoryStore'
import AdvancedSearchDialog from './AdvancedSearchDialog.vue'
import MobileSearchDialog from './MobileSearchDialog.vue'
import SearchHistoryList from './SearchHistoryList.vue'

const props = withDefaults(
  defineProps<{
    halfWidth?: boolean
  }>(),
  {
    halfWidth: false
  }
)

const searchQuery = defineModel<string | null>({ required: true })

const emit = defineEmits<{
  search: [query: string]
}>()

const { smAndDown } = useDisplay()
const searchHistoryStore = useSearchHistoryStore()
const searchFocused = shallowRef(false)
const showMobileSearch = shallowRef(false)
const showAdvancedSearch = shallowRef(false)
let blurTimeout: ReturnType<typeof setTimeout> | null = null

const desktopCardStyle = computed(() => ({ width: props.halfWidth ? '50%' : '100%' }))

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

function applySearch(rawQuery: string | null): void {
  const query = rawQuery?.trim() ?? ''
  searchQuery.value = query === '' ? null : query

  if (query !== '') searchHistoryStore.add(query)

  closeSearchHistory()
  showMobileSearch.value = false
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

function submitAdvancedSearch(filterString: string): void {
  applySearch(filterString)
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
            <template #label>
              <span class="text-caption">Search</span>
            </template>

            <template #append-inner>
              <v-btn
                icon="mdi-tune"
                variant="text"
                size="small"
                aria-label="Advanced search"
                @mousedown.stop.prevent
                @click.stop="openAdvancedSearch"
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
    @search="submitAdvancedSearch"
  />

  <MobileSearchDialog
    v-if="showMobileSearch"
    v-model="showMobileSearch"
    v-model:search-query="searchQuery"
    :history="searchHistoryStore.history"
    @search="applySearch"
    @select-history="selectHistoryItem"
    @remove-history="removeHistoryItem"
    @clear-history="clearSearchHistory"
    @open-advanced-search="openAdvancedSearchFromMobile"
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
