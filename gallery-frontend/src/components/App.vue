<template>
  <v-app
    :style="{
      userSelect:
        scrollbarStore.isDragging || scrollbarStoreInsideAlbum.isDragging ? 'none' : 'auto' // Prevent accidental selection while scrolling.
    }"
  >
    <v-main class="h-screen">
      <NavBar />
      <DropZoneModal v-if="!isMobile()" />
      <HomePage v-if="shouldShowHomePage" :key="routeKey" />
      <router-view v-else />
    </v-main>
    <v-snackbar-queue v-model="messageStore.queue" timeout="2500" />
  </v-app>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, onBeforeMount } from 'vue'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useRerenderStore } from '@/store/rerenderStore'
import { useMessageStore } from '@/store/messageStore'
import DropZoneModal from './Modal/DropZoneModal.vue'
import HomePage from './Page/HomePage.vue'
import isMobile from 'is-mobile'
import { useConstStore } from '@/store/constStore'
import { useFilterStringStore } from '@/store/filterStringStore'
import NavBar from '@/components/NavBar/NavBar.vue'
const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const rerenderStore = useRerenderStore('mainId')
const messageStore = useMessageStore('mainId')
const constStore = useConstStore('mainId')
const route = useRoute()
const filterStringStore = useFilterStringStore()

// Define which route baseNames should show the HomePage component
const allowedBaseNames = ['home', 'all', 'favorite', 'archived', 'trashed', 'albums', 'videos']

// Check if current route should show HomePage
const shouldShowHomePage = computed(() => {
  const baseName = route.meta.baseName
  return typeof baseName === 'string' && allowedBaseNames.includes(baseName)
})

// The routeKey is used to ensure that the router-view reloads the Home.vue component properly.
// Without it, Vue may cache the component for optimization, potentially causing bugs.
const routeKey = computed(() => {
  const search = typeof route.query.search === 'string' ? route.query.search : ''
  const locate = typeof route.query.locate === 'string' ? route.query.locate : ''
  const priorityId = typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
  const reverse = typeof route.query.reverse === 'string' ? route.query.reverse : ''
  const homeKey = rerenderStore.homeKey.toString()
  return `${filterStringStore.filterString}-${search}-${locate}-${priorityId}-${reverse}-${homeKey}`
})

onBeforeMount(async () => {
  // Load the subRowHeightScale from constStore when the app is mounted.
  await constStore.loadSubRowHeightScale()
})
</script>
