<template>
  <v-overlay
    v-model="overlayVisible"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    transition="false"
    :close-on-back="false"
  >
    <Home
      v-if="album !== undefined && basicString !== null"
      isolation-id="subId"
      :basic-string="basicString"
      :search-string="null"
    >
      <template #home-toolbar>
        <HomeIsolatedBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import HomeIsolatedBar from '@/components/NavBar/HomeBars/HomeIsolatedBar.vue'
import { GalleryAlbum } from '@type/types'
import { computed, onBeforeMount, Ref, ref } from 'vue'
import { useCollectionStore } from '@/store/collectionStore'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
const route = useRoute()
const router = useRouter()
const dataStore = useDataStore('mainId')
const album: Ref<GalleryAlbum | undefined> = ref(undefined)
const basicString: Ref<string | null> = ref(null)
const collectionStore = useCollectionStore('subId')

const overlayVisible = computed<boolean>({
  get() {
    // As long as this component exists, the overlay remains open.
    return true
  },
  set(val: boolean) {
    if (!val) {
      // Received a request from Vuetify via ESC to close the overlay.
      if (collectionStore.editModeOn) {
        // In edit mode, first turn off edit mode without closing the overlay.
        collectionStore.editModeOn = false
      } else {
        // Really close the overlay -> navigate back via router.
        router.back()
      }
    }
  }
})

onBeforeMount(() => {
  const hash = route.params.hash
  if (typeof hash === 'string') {
    const index = dataStore.hashMapData.get(hash)
    if (index !== undefined) {
      const data = dataStore.data.get(index)
      if (data?.type === 'album') {
        album.value = data
      }
    }
  }
  const album_id = route.params.hash
  if (typeof album_id === 'string') {
    basicString.value = `and(album:"${album_id}", trashed:false)`
  }
})
</script>
