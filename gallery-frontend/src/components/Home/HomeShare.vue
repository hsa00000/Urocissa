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
import ShareLoginModal from '@/components/Modal/ShareLoginModal.vue' // Import
import { onBeforeMount, ref, Ref } from 'vue'
import { useShareStore } from '@/store/shareStore'

const route = useRoute()
const albumId: Ref<string | undefined> = ref(undefined)
const shareId: Ref<string | undefined> = ref(undefined)
const basicString: Ref<string | undefined> = ref(undefined)
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)

const shareStore = useShareStore('mainId')

onBeforeMount(() => {
  const albumIdOpt = route.params.albumId
  const shareIdOpt = route.params.shareId

  // Reset store state on mount
  shareStore.isAuthFailed = false
  shareStore.password = null

  if (typeof albumIdOpt === 'string' && typeof shareIdOpt === 'string') {
    albumId.value = albumIdOpt
    shareId.value = shareIdOpt
    basicString.value = `and(not(trashed:true), album:"${albumIdOpt}")`

    shareStore.albumId = albumIdOpt
    shareStore.shareId = shareIdOpt
  } else {
    console.error(`(albumId, shareId) is (${albumId.value}, ${shareId.value})`)
  }
  searchString.value = route.query.search
})
</script>
