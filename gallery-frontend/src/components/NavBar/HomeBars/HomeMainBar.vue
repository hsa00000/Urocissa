<script setup lang="ts">
import { computed, inject, shallowRef, watchEffect, type Ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDisplay, useTheme } from 'vuetify'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useAlbumStore } from '@/store/albumStore'
import { useConstStore } from '@/store/constStore'
import { useUploadStore } from '@/store/uploadStore'
import { useMessageStore } from '@/store/messageStore'
import { useSavedSearchStore } from '@/store/savedSearchStore'
import { useRerenderStore } from '@/store/rerenderStore'
import EditBar from '@/components/NavBar/EditBar.vue'
import HomeBarTemplate from '@/components/NavBar/HomeBars/HomeBarTemplate.vue'
import GallerySearchControl from '@/components/Search/GallerySearchControl.vue'
import { createGallerySearchRouteUpdate } from '@/components/Search/gallerySearchRoute'
import SavedSearchNameDialog from '@/components/SavedSearch/SavedSearchNameDialog.vue'
import {
  canSaveSearch,
  getSavedSearchContext,
  SAVED_SEARCH_ADDED_MESSAGE
} from '@/components/SavedSearch/savedSearchRoute'
import BtnCreateAlbum from '@Menu/MenuButton/BtnCreateAlbum.vue'
import type { GallerySortOrder, SavedSearchContext } from '@/type/types'
import {
  parseGallerySortOrder,
  type GallerySearchSubmission
} from '@/script/utils/gallerySort'

const showDrawer = inject<Ref<boolean>>('showDrawer')
const albumStore = useAlbumStore('mainId')
const filterStore = useFilterStore('mainId')
const constStore = useConstStore('mainId')
const collectionStore = useCollectionStore('mainId')
const uploadStore = useUploadStore('mainId')
const messageStore = useMessageStore('mainId')
const savedSearchStore = useSavedSearchStore()
const rerenderStore = useRerenderStore('mainId')
const vuetifyTheme = useTheme()
const { smAndDown } = useDisplay()
const route = useRoute()
const router = useRouter()
const searchQuery = shallowRef<string | null>(null)
const showSaveSearchDialog = shallowRef(false)
const pendingSavedSearch = shallowRef<{
  context: SavedSearchContext
  query: string
  sortOrder: GallerySortOrder
} | null>(null)
const loading = shallowRef(false)
const isLevelOne = computed(() => route.meta.level === 1)
const isUploadPage = computed(() => route.name === 'upload')
const showUploadProgress = computed(
  () => uploadStore.currentRunTotalCount > 0 && uploadStore.hasActiveWork
)
const showCompletedUploadError = computed(
  () => uploadStore.currentRunIsComplete && uploadStore.currentRunErrorCount > 0
)
const uploadProgress = computed(() => {
  const percent = uploadStore.currentRunProgressPercent
  return uploadStore.hasActiveWork && percent >= 100 ? 99 : percent
})
const uploadProgressColor = computed(() =>
  uploadStore.currentRunErrorCount > 0 ? 'error' : 'primary'
)
const savedSearchContext = computed(() => getSavedSearchContext(route))
const currentSortOrder = computed(() => parseGallerySortOrder(route.query.sort))
const canSaveCurrentSearch = computed(() =>
  canSaveSearch(savedSearchContext.value, searchQuery.value)
)
const suggestedSavedSearchName = computed(() =>
  Array.from(pendingSavedSearch.value?.query ?? '')
    .slice(0, 80)
    .join('')
)
const existingSavedSearchNames = computed(() =>
  savedSearchStore.searches.map((search) => search.name)
)

const themeIsLight = computed<boolean>({
  get: () => constStore.theme === 'light',
  set: () => {
    constStore.toggleTheme(vuetifyTheme).catch((error: unknown) => {
      console.error('Failed to update theme (via InfoBar):', error)
    })
  }
})

function toggleDrawer(): void {
  if (showDrawer === undefined) return
  showDrawer.value = !showDrawer.value
}

async function applySearchState({
  query: rawQuery,
  sortOrder
}: GallerySearchSubmission): Promise<void> {
  const { normalizedSearch, routeQuery } = createGallerySearchRouteUpdate(
    route.query,
    'search',
    { query: rawQuery, sortOrder }
  )
  filterStore.searchString = normalizedSearch === '' ? null : normalizedSearch

  const location = {
    path: route.path,
    query: routeQuery
  }
  if (
    sortOrder === 'random' &&
    router.resolve(location).fullPath === route.fullPath
  ) {
    rerenderStore.rerenderHome()
    return
  }

  await router.replace(location)
}

async function handleSearch(query: string): Promise<void> {
  await applySearchState({ query, sortOrder: currentSortOrder.value })
}

async function handleAdvancedSearch(
  submission: GallerySearchSubmission
): Promise<void> {
  await applySearchState(submission)
}

async function handleSort(sortOrder: GallerySortOrder): Promise<void> {
  await applySearchState({
    query: searchQuery.value ?? '',
    sortOrder
  })
}

function navigateToUpload(): void {
  void router.push({ name: 'upload' })
}

function openSaveSearchDialog(query: string): void {
  const context = savedSearchContext.value
  const normalizedQuery = query.trim()
  if (context === null || normalizedQuery === '') return

  pendingSavedSearch.value = {
    context,
    query: normalizedQuery,
    sortOrder: currentSortOrder.value
  }
  showSaveSearchDialog.value = true
}

async function saveSearch(name: string): Promise<void> {
  const pending = pendingSavedSearch.value
  if (pending === null) return

  const succeeded = await savedSearchStore.create({
    name,
    context: pending.context,
    query: pending.query,
    sortOrder: pending.sortOrder
  })
  if (!succeeded) return

  showSaveSearchDialog.value = false
  pendingSavedSearch.value = null
  messageStore.success(SAVED_SEARCH_ADDED_MESSAGE)
}

watchEffect(() => {
  searchQuery.value =
    typeof filterStore.searchString === 'string' ? filterStore.searchString : null
})
</script>

<template>
  <HomeBarTemplate isolation-id="mainId">
    <template #content>
      <v-toolbar
        v-if="!collectionStore.editModeOn"
        class="position-relative bg-surface"
      >
        <v-btn
          v-if="isLevelOne"
          icon="mdi-menu"
          aria-label="Open navigation drawer"
          @click="toggleDrawer"
        />
        <v-btn
          v-else
          icon="mdi mdi-arrow-left"
          :to="albumStore.leaveAlbumPath ? albumStore.leaveAlbumPath : '/'"
        />

        <v-card
          v-if="route.meta.level === 3 && typeof route.params.hash === 'string'"
          elevation="0"
          class="w-50"
        >
          <v-card-title class="text-truncate">
            {{ albumStore.albums.get(route.params.hash) }}
          </v-card-title>
        </v-card>

        <GallerySearchControl
          v-if="!isUploadPage"
          v-model="searchQuery"
          :sort-order="currentSortOrder"
          :half-width="route.meta.level === 3"
          :can-save="canSaveCurrentSearch"
          :desktop-end-gap="isLevelOne ? 0 : 10"
          :desktop-layout="isLevelOne ? 'center-quarter' : 'flow'"
          @search="handleSearch"
          @advanced-search="handleAdvancedSearch"
          @sort="handleSort"
          @save="openSaveSearchDialog"
        />
        <v-spacer v-if="isUploadPage || (isLevelOne && !smAndDown)" />

        <v-btn
          v-if="isLevelOne"
          class="d-none d-md-flex"
          :icon="themeIsLight ? 'mdi-weather-sunny' : 'mdi-weather-night'"
          :aria-label="themeIsLight ? 'Switch to dark theme' : 'Switch to light theme'"
          @click="themeIsLight = !themeIsLight"
        />
        <BtnCreateAlbum v-if="isLevelOne" v-model="loading" />
        <div v-if="isLevelOne" class="upload-btn-container">
          <v-btn
            icon
            class="upload-btn"
            :color="isUploadPage ? 'primary' : undefined"
            aria-label="Open upload page"
            data-testid="navbar-upload-btn"
            @click="navigateToUpload"
          >
            <v-icon icon="mdi-upload" />

            <v-progress-circular
              v-if="showUploadProgress"
              :model-value="uploadProgress"
              :color="uploadProgressColor"
              size="40"
              width="3"
              class="upload-progress-overlay"
              aria-hidden="true"
            />

            <v-progress-circular
              v-else-if="showCompletedUploadError"
              :model-value="100"
              color="error"
              size="40"
              width="3"
              class="upload-progress-overlay"
              aria-hidden="true"
            />
          </v-btn>
        </div>
      </v-toolbar>

      <EditBar v-else />
    </template>
  </HomeBarTemplate>

  <SavedSearchNameDialog
    v-if="pendingSavedSearch !== null"
    v-model="showSaveSearchDialog"
    title="Save Search"
    :initial-name="suggestedSavedSearchName"
    :existing-names="existingSavedSearchNames"
    :loading="savedSearchStore.mutating"
    @submit="saveSearch"
  />
</template>

<style scoped>
.upload-btn-container {
  position: relative;
  display: inline-flex;
  align-items: center;
  justify-content: center;
}

.upload-btn {
  position: relative;
}

.upload-progress-overlay {
  position: absolute;
  top: 50%;
  left: 50%;
  transform: translate(-50%, -50%);
  pointer-events: none;
}
</style>
