<template>
  <v-app
    :style="{
      userSelect:
        scrollbarStore.isDragging || scrollbarStoreInsideAlbum.isDragging ? 'none' : 'auto' // Prevent accidental selection while scrolling.
    }"
  >
    <v-main class="h-screen">

      <DropZoneModal v-if="!isMobile()" />
      <HomePage v-if="shouldShowHomePage" :key="routeKey" :filter-string="filterString" />
      <router-view v-else v-slot="{ Component }">
        <component :is="Component" :filter-string="filterString" />
      </router-view>
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
import { virtualRouteNames } from '@/route/pageReturnType'

const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const rerenderStore = useRerenderStore('mainId')
const messageStore = useMessageStore()
const constStore = useConstStore()
const route = useRoute()

const filterString = computed(() => {
  const baseName = route.meta.baseName
  if (typeof baseName !== 'string') return ''
  switch (baseName) {
    case 'home':
      return 'and(not(tag:"_archived"), not(tag:"_trashed"))'
    case 'all':
      return 'not(tag:"_trashed")'
    case 'favorite':
      return 'and(tag:"_favorite", not(tag:"_trashed"))'
    case 'archived':
      return 'and(tag:"_archived", not(tag:"_trashed"))'
    case 'trashed':
      return 'and(tag:"_trashed")'
    case 'albums':
      return 'and(type:"album", not(tag:"_trashed"))'
    case 'videos':
      return 'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
    case 'share': {
      const albumIdOpt = route.params.albumId
      return typeof albumIdOpt === 'string' ? `and(not(tag:"_trashed"), album:"${albumIdOpt}")` : ''
    }
    default:
      return ''
  }
})

// Check if current route should show HomePage
const shouldShowHomePage = computed(() => {
  const baseName = route.meta.baseName
  return typeof baseName === 'string' && (virtualRouteNames as readonly string[]).includes(baseName)
})

// The routeKey is used to ensure that the router-view reloads the Home.vue component properly.
// Without it, Vue may cache the component for optimization, potentially causing bugs.
const routeKey = computed(() => {
  const search = typeof route.query.search === 'string' ? route.query.search : ''
  const locate = typeof route.query.locate === 'string' ? route.query.locate : ''
  const priorityId = typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
  const reverse = typeof route.query.reverse === 'string' ? route.query.reverse : ''
  const homeKey = rerenderStore.homeKey.toString()
  return `${filterString.value}-${search}-${locate}-${priorityId}-${reverse}-${homeKey}`
})

onBeforeMount(async () => {
  // Load the subRowHeightScale from constStore when the app is mounted.
  await constStore.loadSubRowHeightScale()
})
</script>
