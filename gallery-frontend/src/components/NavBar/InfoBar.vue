<template>
  <v-toolbar
    :style="{
      backgroundColor: '#212121'
    }"
  >
    <v-btn v-if="!route.meta.isReadPage" @click="showDrawer = !showDrawer" icon="mdi-menu"> </v-btn>
    <v-btn
      v-else
      icon="mdi mdi-arrow-left"
      :to="albumStore.leaveAlbumPath ? albumStore.leaveAlbumPath : '/'"
    ></v-btn>
    <v-card
      v-if="
        route.meta.isReadPage && !route.meta.isViewPage && typeof route.params.hash === 'string'
      "
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
        width: `${route.meta.isReadPage ? '50%' : '100%'}`
      }"
    >
      <v-card-text class="pa-0">
        <v-text-field
          id="nav-search-input"
          rounded
          class="ma-0"
          v-model="searchQuery"
          bg-color="grey-darken-2"
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

    <BtnCreateAlbum v-if="!route.meta.isReadPage" v-model="loading" />
    <v-btn
      v-if="!route.meta.isReadPage"
      icon="mdi-upload"
      :loading="loading"
      @click="uploadStore.triggerFileInput"
    />
  </v-toolbar>
</template>

<script setup lang="ts">
import { inject, Ref, ref, watchEffect } from 'vue'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useSearchStringStore } from '@/store/searchStringStore'
import { useUploadStore } from '@/store/uploadStore'
import { useAlbumStore } from '@/store/albumStore'
import BtnCreateAlbum from '@Menu/MenuButton/BtnCreateAlbum.vue'

const showDrawer = inject('showDrawer')

const albumStore = useAlbumStore('mainId')
const uploadStore = useUploadStore('mainId')
const searchStringStore = useSearchStringStore('mainId')

const route = useRoute()
const router = useRouter()
const searchQuery: Ref<LocationQueryValue | LocationQueryValue[] | undefined> = ref(null)
const loading = ref(false)

const handleSearch = async () => {
  searchStringStore.searchString = searchQuery.value
  await router.replace({
    path: route.path,
    query: { search: searchQuery.value }
  })
}

watchEffect(() => {
  searchQuery.value = searchStringStore.searchString
})
</script>
