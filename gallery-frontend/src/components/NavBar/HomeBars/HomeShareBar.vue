<script setup lang="ts">
import { computed, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useCollectionReloadStore } from '@/store/collectionReloadStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useShareStore } from '@/store/shareStore'
import EditBarShare from '@/components/NavBar/EditBarShare.vue'
import HomeBarTemplate from '@/components/NavBar/HomeBars/HomeBarTemplate.vue'
import GallerySearchControl from '@/components/Search/GallerySearchControl.vue'
import { createGallerySearchRouteUpdate } from '@/components/Search/gallerySearchRoute'
import { parseGallerySortOrder, type GallerySearchSubmission } from '@/script/utils/gallerySort'
import type { GallerySortOrder } from '@/type/types'

const filterStore = useFilterStore('mainId')
const shareStore = useShareStore('mainId')
const collectionStore = useCollectionStore('mainId')
const collectionReloadStore = useCollectionReloadStore('mainId')

const route = useRoute()
const router = useRouter()
const searchQuery = shallowRef<string | null>(null)
const currentSortOrder = computed(() => parseGallerySortOrder(route.query.sort))

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
  if (sortOrder === 'random' && router.resolve(location).fullPath === route.fullPath) {
    collectionReloadStore.requestMainCollectionReload()
    return
  }

  await router.replace(location)
}

async function handleSearch(query: string): Promise<void> {
  await applySearchState({ query, sortOrder: currentSortOrder.value })
}

async function handleAdvancedSearch(submission: GallerySearchSubmission): Promise<void> {
  await applySearchState(submission)
}

async function handleSort(sortOrder: GallerySortOrder): Promise<void> {
  await applySearchState({
    query: searchQuery.value ?? '',
    sortOrder
  })
}

watch(
  () => route.query.search,
  (query) => {
    searchQuery.value = typeof query === 'string' ? query : null
  },
  { immediate: true }
)
</script>

<template>
  <HomeBarTemplate isolation-id="mainId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="position-relative bg-surface">
        <v-card elevation="0" class="w-50">
          <v-card-title>{{ shareStore.resolvedShare?.albumTitle }}</v-card-title>
        </v-card>

        <GallerySearchControl
          v-model="searchQuery"
          :sort-order="currentSortOrder"
          desktop-layout="center-quarter"
          :show-save-action="false"
          :include-album-media-type="false"
          :desktop-end-gap="0"
          @search="handleSearch"
          @advanced-search="handleAdvancedSearch"
          @sort="handleSort"
        />

        <v-spacer />
      </v-toolbar>

      <EditBarShare v-else />
    </template>
  </HomeBarTemplate>
</template>
