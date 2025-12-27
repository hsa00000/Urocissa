<template>
  <HomeBarTemplate isolation-id="mainId">
    <template #content>
      <v-toolbar v-if="!collectionStore.editModeOn" class="bg-surface">
        <v-btn v-if="route.meta.level === 1" @click="showDrawer = !showDrawer" icon="mdi-menu">
        </v-btn>
        <v-btn
          v-else
          icon="mdi mdi-arrow-left"
          :to="albumStore.leaveAlbumPath ? albumStore.leaveAlbumPath : '/'"
        ></v-btn>
        <v-card
          v-if="route.meta.level === 3 && typeof route.params.hash === 'string'"
          elevation="0"
          class="w-50"
        >
          <v-card-title class="text-truncate">
            {{ albumStore.albums.get(route.params.hash) }}
          </v-card-title>
        </v-card>
        <v-card elevation="0" :style="{ width: `${route.meta.level === 3 ? '50%' : '100%'}` }">
          <v-card-text class="pa-0 bg-surface">
            <v-text-field
              id="nav-search-input"
              rounded
              class="ma-0"
              v-model="searchQuery"
              bg-color="surface-light"
              @click:prepend-inner="handleSearch"
              @click:clear="handleSearch"
              @keyup.enter="handleSearch"
              clearable
              persistent-clear
              variant="solo"
              flat
              prepend-inner-icon="mdi-magnify"
              single-line
              hide-details
              style="margin-right: 10px"
            >
              <template #label>
                <span class="text-caption">Search</span>
              </template>
            </v-text-field>
          </v-card-text>
        </v-card>

        <v-btn
          v-if="route.meta.level === 1"
          :icon="themeIsLight ? 'mdi-weather-sunny' : 'mdi-weather-night'"
          @click="themeIsLight = !themeIsLight"
        />
        <BtnCreateAlbum v-if="route.meta.level === 1" v-model="loading" />
        <v-btn
          v-if="route.meta.level === 1"
          icon="mdi-upload"
          :loading="loading"
          @click="uploadStore.triggerFileInput"
        />
      </v-toolbar>
      <EditBar v-else />
    </template>
  </HomeBarTemplate>
</template>

<script setup lang="ts">
import { computed, inject, Ref, ref, watchEffect } from 'vue'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useCollectionStore } from '@/store/collectionStore'
import { useFilterStore } from '@/store/filterStore'
import { useUploadStore } from '@/store/uploadStore'
import { useAlbumStore } from '@/store/albumStore'
import { useConstStore } from '@/store/constStore'
import EditBar from '@/components/NavBar/EditBar.vue'
import BtnCreateAlbum from '@Menu/MenuButton/BtnCreateAlbum.vue'
import { useTheme } from 'vuetify'
import HomeBarTemplate from '@/components/NavBar/HomeBars/HomeBarTemplate.vue'

const showDrawer = inject('showDrawer')

const albumStore = useAlbumStore('mainId')
const uploadStore = useUploadStore('mainId')
const filterStore = useFilterStore('mainId')
const constStore = useConstStore('mainId')
const vuetifyTheme = useTheme()

const themeIsLight = computed<boolean>({
  get: () => constStore.theme === 'light',
  set: () => {
    constStore.toggleTheme(vuetifyTheme).catch((err: unknown) => {
      console.error('Failed to update theme (via InfoBar):', err)
    })
  }
})

const route = useRoute()
const router = useRouter()
const searchQuery: Ref<LocationQueryValue | LocationQueryValue[] | undefined> = ref(null)
const loading = ref(false)

const handleSearch = async () => {
  filterStore.searchString = searchQuery.value
  await router.replace({
    path: route.path,
    query: { search: searchQuery.value }
  })
}

watchEffect(() => {
  searchQuery.value = filterStore.searchString
})

const collectionStore = useCollectionStore('mainId')
</script>

<style scoped>
.no-select {
  user-select: none;
}
.no-select * {
  user-select: none;
}
</style>
