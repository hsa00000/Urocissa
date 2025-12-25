<template>
  <Home
    v-if="basicString !== undefined"
    :key="`${shareStore.password}`"
    isolation-id="mainId"
    :basic-string="basicString"
    :search-string="searchString"
  >
    <template #home-toolbar> <HomeShareBar /> </template>
  </Home>

  <ShareLoginModal />
</template>
<script setup lang="ts">
import { LocationQueryValue, useRoute } from 'vue-router'
import Home from './Home.vue'
import HomeShareBar from '@/components/NavBar/HomeBars/HomeShareBar.vue'
import ShareLoginModal from '@/components/Modal/ShareLoginModal.vue'
import { onBeforeMount, ref, Ref, watch } from 'vue'
import { useShareStore } from '@/store/shareStore'
import { getShareInfo } from '@/db/db'

const route = useRoute()
const albumId: Ref<string | undefined> = ref(undefined)
const shareId: Ref<string | undefined> = ref(undefined)
const basicString: Ref<string | undefined> = ref(undefined)
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)

const shareStore = useShareStore('mainId')

onBeforeMount(async () => {
  searchString.value = route.query.search

  const albumIdOpt = route.params.albumId
  const shareIdOpt = route.params.shareId

  // Reset store state on mount
  shareStore.isAuthFailed = false
  shareStore.isLinkExpired = false

  if (typeof albumIdOpt === 'string' && typeof shareIdOpt === 'string') {
    albumId.value = albumIdOpt
    shareId.value = shareIdOpt
    basicString.value = `and(trashed:false, album:"${albumIdOpt}")`

    shareStore.albumId = albumIdOpt
    shareStore.shareId = shareIdOpt

    const savedInfo = await getShareInfo(albumIdOpt, shareIdOpt)
    if (savedInfo && savedInfo.password) {
      shareStore.password = savedInfo.password
    }

    // Sync to IndexedDB for Service Worker
    await shareStore.syncShareInfoToIndexedDB()
  } else {
    console.error(`(albumId, shareId) is (${albumId.value}, ${shareId.value})`)
  }
})

// Watch for password changes and sync to IndexedDB
watch(
  () => shareStore.password,
  async () => {
    await shareStore.syncShareInfoToIndexedDB()
  }
)

// Note: We intentionally do NOT clear IndexedDB on unmount
// because other tabs may still be using the same share.
// Each share has its own key (albumId_shareId) in IndexedDB,
// so there's no pollution between different shares.
</script>
