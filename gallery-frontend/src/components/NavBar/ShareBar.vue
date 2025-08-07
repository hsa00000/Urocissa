<template>
  <v-toolbar
    flat
    height="2"
    class="no-select"
    :style="{
      backgroundColor: '#212121'
    }"
  >
  </v-toolbar>
  <v-toolbar
    class="position-relative"
    :style="{
      backgroundColor: '#212121'
    }"
  >
    <v-card elevation="0" class="w-50">
      <v-card-title> {{ shareStore.resolvedShare?.albumTitle }} </v-card-title>
    </v-card>
    <v-card
      elevation="0"
      :style="{
        width: '50%'
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
    <v-spacer></v-spacer>
  </v-toolbar>
  <ProgessBar isolation-id="mainId" />
</template>
<script setup lang="ts">
import ProgessBar from '@/components/NavBar/ProgessBar.vue'
import { Ref, ref, watchEffect } from 'vue'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useSearchStringStore } from '@/store/searchStringStore'
import { useShareStore } from '@/store/shareStore'

const searchStringStore = useSearchStringStore('mainId')
const shareStore = useShareStore('mainId')

const route = useRoute()
const router = useRouter()
const searchQuery: Ref<LocationQueryValue | LocationQueryValue[] | undefined> = ref(null)

const handleSearch = async () => {
  searchStringStore.searchString = searchQuery.value
  await router.replace({
    path: route.path,
    query: { search: searchQuery.value }
  })
}

watchEffect(() => {
  searchQuery.value = searchStringStore.searchString
  console.log('shareStore.resolvedShare?.albumTitle is', shareStore.resolvedShare?.albumTitle)
})
</script>
