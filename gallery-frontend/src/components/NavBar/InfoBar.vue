<template>
  <v-app-bar elevation="0">
    <v-btn v-if="route.meta.level === 1" @click="showDrawer = !showDrawer" icon="mdi-menu"> </v-btn>
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
    <v-card
      elevation="0"
      :style="{
        width: `${route.meta.level === 3 ? '50%' : '100%'}`
      }"
    >
      <v-card-text class="pa-0">
        <v-text-field
          id="nav-search-input"
          v-model="searchQuery"
          variant="solo-filled"
          flat
          clearable
          persistent-clear
          prepend-inner-icon="mdi-magnify"
          single-line
          hide-details
          rounded
          class="ma-0"
          @click:prepend-inner="handleSearch"
          @click:clear="handleSearch"
          @keyup.enter="handleSearch"
        >
          <template #label>
            <span class="text-caption">Search</span>
          </template>
        </v-text-field>
      </v-card-text>
    </v-card>

    <BtnCreateAlbum v-if="route.meta.level === 1" v-model="loading" />
    <v-btn
      v-if="route.meta.level === 1"
      icon="mdi-upload"
      :loading="loading"
      @click="uploadStore.triggerFileInput"
    />
  </v-app-bar>
</template>

<script setup lang="ts">
import { inject, Ref, ref, watchEffect } from 'vue'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useFilterStore } from '@/store/filterStore'
import { useUploadStore } from '@/store/uploadStore'
import { useAlbumStore } from '@/store/albumStore'
import BtnCreateAlbum from '@Menu/MenuButton/BtnCreateAlbum.vue'

const showDrawer = inject('showDrawer')

const albumStore = useAlbumStore('mainId')
const uploadStore = useUploadStore('mainId')
const filterStore = useFilterStore('mainId')

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
</script>
