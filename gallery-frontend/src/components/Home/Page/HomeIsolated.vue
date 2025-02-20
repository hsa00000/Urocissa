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
    <Home v-if="album !== undefined" isolation-id="subId" :filter-determined-by-query="null">
      <template #reading-bar>
        <ReadingBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import ReadingBar from '@/components/NavBar/ReadingBar.vue'
import { Album } from '@/script/common/types'
import { onMounted, Ref, ref } from 'vue'
import { useRoute } from 'vue-router'
import { useDataStore } from '@/store/dataStore'

const route = useRoute()
const dataStore = useDataStore('mainId')
const album: Ref<Album | undefined> = ref(undefined)

onMounted(() => {
  const hash = route.params.hash
  if (typeof hash === 'string') {
    const index = dataStore.hashMapData.get(hash)
    if (index !== undefined) {
      album.value = dataStore.data.get(index)?.album
    }
  }
})
</script>
