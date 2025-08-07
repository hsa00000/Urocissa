<template>
  <v-app
    :style="{
      userSelect:
        scrollbarStore.isDragging || scrollbarStoreInsideAlbum.isDragging ? 'none' : 'auto' // Prevent accidental selection while scrolling.
    }"
  >
    <v-main class="h-screen">
      <DropZoneModal v-if="!isMobile()" />
      <UniversalPage v-if="shouldShowUniversalPage" :key="routeKey" />
    </v-main>
    <v-snackbar-queue v-model="messageStore.queue" timeout="2500" />
  </v-app>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { computed, onBeforeMount, watch } from 'vue'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useRerenderStore } from '@/store/rerenderStore'
import { useMessageStore } from '@/store/messageStore'
import DropZoneModal from './Modal/DropZoneModal.vue'
import UniversalPage from './Page/UniversalPage.vue'
import isMobile from 'is-mobile'
import { useConstStore } from '@/store/constStore'
import { useBasicStringStore } from '@/store/basicStringStore'
const scrollbarStore = useScrollbarStore('mainId')
const scrollbarStoreInsideAlbum = useScrollbarStore('subId')
const rerenderStore = useRerenderStore('mainId')
const messageStore = useMessageStore('mainId')
const constStore = useConstStore('mainId')
const route = useRoute()
const basicStringStore = useBasicStringStore('mainId')
// The routeKey is used to ensure that the router-view reloads the Home.vue component properly.
// Without it, Vue may cache the component for optimization, potentially causing bugs.
const routeKey = computed(() => {
  const search = typeof route.query.search === 'string' ? route.query.search : ''
  const locate = typeof route.query.locate === 'string' ? route.query.locate : ''
  const priorityId = typeof route.query.priority_id === 'string' ? route.query.priority_id : ''
  const reverse = typeof route.query.reverse === 'string' ? route.query.reverse : ''
  const homeKey = rerenderStore.homeKey.toString()
  return `${basicStringStore.basicString}-${search}-${locate}-${priorityId}-${reverse}-${homeKey}`
})

// Check if should show UniversalPage based on route query type
const shouldShowUniversalPage = computed(() => {
  const type = route.query.type as string
  const validTypes = ['home', 'all', 'favorite', 'archived', 'trashed', 'albums', 'videos']
  return validTypes.includes(type)
})

// 监听 query type 的变化，更新 basicStringStore
watch(
  () => route.query.type,
  (newType) => {
    // 如果 query type 变成 null 则不要更新 basicStringStore
    if (newType !== null && newType !== undefined) {
      const type = newType as string
      let newBasicString: string

      switch (type) {
        case 'home':
          newBasicString = 'and(not(tag:"_archived"), not(tag:"_trashed"))'
          break
        case 'all':
          newBasicString = 'not(tag:"_trashed")'
          break
        case 'favorite':
          newBasicString = 'and(tag:"_favorite", not(tag:"_trashed"))'
          break
        case 'archived':
          newBasicString = 'and(tag:"_archived", not(tag:"_trashed"))'
          break
        case 'trashed':
          newBasicString = 'and(tag:"_trashed")'
          break
        case 'albums':
          newBasicString = 'and(type:"album", not(tag:"_trashed"))'
          break
        case 'videos':
          newBasicString = 'and(type:"video", not(tag:"_archived"), not(tag:"_trashed"))'
          break
        default:
          newBasicString = 'and(not(tag:"_archived"), not(tag:"_trashed"))'
          break
      }

      basicStringStore.basicString = newBasicString
    }
  },
  { immediate: true }
)

onBeforeMount(async () => {
  // Load the subRowHeightScale from constStore when the app is mounted.
  await constStore.loadSubRowHeightScale()
})
</script>
