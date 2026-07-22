<script setup lang="ts">
import { computed, inject, shallowRef, watchEffect, type Ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useTheme } from 'vuetify'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useUploadStore } from '@/store/uploadStore'
import { useAlbumStore } from '@/store/albumStore'
import { useConstStore } from '@/store/constStore'
import EditBar from '@/components/NavBar/EditBar.vue'
import HomeBarTemplate from '@/components/NavBar/HomeBars/HomeBarTemplate.vue'
import GallerySearchControl from '@/components/Search/GallerySearchControl.vue'
import BtnCreateAlbum from '@Menu/MenuButton/BtnCreateAlbum.vue'

const showDrawer = inject<Ref<boolean>>('showDrawer')
const albumStore = useAlbumStore('mainId')
const uploadStore = useUploadStore('mainId')
const filterStore = useFilterStore('mainId')
const constStore = useConstStore('mainId')
const collectionStore = useCollectionStore('mainId')
const vuetifyTheme = useTheme()
const route = useRoute()
const router = useRouter()
const searchQuery = shallowRef<string | null>(null)
const loading = shallowRef(false)

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

async function handleSearch(query: string): Promise<void> {
  filterStore.searchString = query === '' ? null : query

  const nextQuery = { ...route.query }
  if (query === '') {
    delete nextQuery.search
  } else {
    nextQuery.search = query
  }

  await router.replace({
    path: route.path,
    query: nextQuery
  })
}

watchEffect(() => {
  searchQuery.value =
    typeof filterStore.searchString === 'string' ? filterStore.searchString : null
})
</script>

<template>
  <HomeBarTemplate isolation-id="mainId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="bg-surface">
        <v-btn v-if="route.meta.level === 1" icon="mdi-menu" @click="toggleDrawer" />
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
          v-model="searchQuery"
          :half-width="route.meta.level === 3"
          @search="handleSearch"
        />

        <v-btn
          v-if="route.meta.level === 1"
          class="d-none d-md-flex"
          :icon="themeIsLight ? 'mdi-weather-sunny' : 'mdi-weather-night'"
          @click="themeIsLight = !themeIsLight"
        />
        <BtnCreateAlbum v-if="route.meta.level === 1" v-model="loading" />
        <v-btn
          v-if="route.meta.level === 1"
          icon="mdi-upload"
          :loading="loading"
          @click="uploadStore.triggerFileInput(undefined)"
        />
      </v-toolbar>

      <EditBar v-else />
    </template>
  </HomeBarTemplate>
</template>
