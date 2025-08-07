<template>
  <v-overlay
    :model-value="true"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    transition="false"
    :close-on-back="false"
    persistent
  >
    <Home
      v-if="album !== undefined && filterString !== null"
      isolation-id="subId"
      :filter-string="filterString"
      :search-string="null"
    >
      <template #reading-bar>
        <ReadingBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import ReadingBar from '@/components/NavBar/ReadingBar.vue'
import { Album } from '@type/types'
import { onBeforeMount, Ref, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useDataStore } from '@/store/dataStore'

const route = useRoute()
const dataStore = useDataStore('mainId')
const album: Ref<Album | undefined> = ref(undefined)
const filterString: Ref<string | null> = ref(null)

onBeforeMount(() => {
  const hash = route.params.hash
  if (typeof hash === 'string') {
    const index = dataStore.hashMapData.get(hash)
    if (index !== undefined) {
      album.value = dataStore.data.get(index)?.album
    }
  }
  const album_id = route.params.hash
  if (typeof album_id === 'string') {
    filterString.value = `and(album:"${album_id}", not(tag:"_trashed"))`
  }
})
</script>
