<template>
  <v-overlay
    :model-value="true"
    @update:model-value="
      (val) => {
        if (val === false) {
          router.back()
        }
      }
    "
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
import { Album } from '@type/types'
import { onBeforeMount, Ref, ref } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
const route = useRoute()
const router = useRouter()
const dataStore = useDataStore('mainId')
const album: Ref<Album | undefined> = ref(undefined)
const basicString: Ref<string | null> = ref(null)

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
    basicString.value = `and(album:"${album_id}", not(tag:"_trashed"))`
  }
})
</script>
