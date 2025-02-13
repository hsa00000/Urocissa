<template>
  <v-app
    :style="{
      userSelect:
        scrollbarStore.isDragging || scrollbarStoreInsideAlbum.isDragging ? 'none' : 'auto' // Prevent accidental selection while scrolling.
    }"
  >
    <v-main class="h-screen">
      <DropZoneModal v-if="!isMobile()" />
      <router-view v-slot="{ Component }" :key="routeKey">
        <component :is="Component" />
      </router-view>
    </v-main>
    <NotificationWarn />
  </v-app>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed } from 'vue'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useRerenderStore } from '@/store/rerenderStore'
import NotificationWarn from '@/components/NotificationWarn.vue'
import DropZoneModal from './Modal/DropZoneModal.vue'
import isMobile from 'is-mobile'
const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const rerenderStore = useRerenderStore('mainId')
const route = useRoute()

const currentPage = computed(() => {
  if (route.path.startsWith('/favorite')) {
    return 'favorite'
  } else if (route.path.startsWith('/archived')) {
    return 'archived'
  } else if (route.path.startsWith('/all')) {
    return 'all'
  } else if (route.path.startsWith('/home')) {
    return 'home'
  } else {
    return 'default'
  }
})

// The routeKey is used to ensure that the router-view reloads the Home.vue component properly.
// Without it, Vue may cache the component for optimization, potentially causing bugs.
const routeKey = computed(() => {
  const search = typeof route.query.search === 'string' ? route.query.search : ''
  const locate = typeof route.query.locate === 'string' ? route.query.locate : ''
  const priorityId = typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
  const reverse = typeof route.query.reverse === 'string' ? route.query.reverse : ''
  const homeKey = rerenderStore.homeKey.toString()
  return `${currentPage.value}-${search}-${locate}-${priorityId}-${reverse}-${homeKey}`
})
</script>
