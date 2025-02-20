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
      v-if="basicString !== undefined"
      isolation-id="shareId"
      :filter-determined-by-query="generateJsonString(basicString)"
    >
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import { useRoute } from 'vue-router'
import Home from './Home.vue'
import { generateJsonString } from '@/script/lexer/generateJson'

import { onMounted, ref, Ref } from 'vue'

const route = useRoute()
const albumId: Ref<string | undefined> = ref(undefined)
const shareId: Ref<string | undefined> = ref(undefined)
const basicString: Ref<string | undefined> = ref(undefined)

onMounted(() => {
  const albumIdOpt = route.params.albumId
  const shareIdOpt = route.params.shareId
  if (typeof albumIdOpt === 'string' && typeof shareIdOpt === 'string') {
    albumId.value = albumIdOpt
    shareId.value = shareIdOpt
    basicString.value = `and(not(tag:"_trashed"), album:"${albumIdOpt}")`
  } else {
    console.error(`(albumId, shareId) is (${albumId.value}, ${shareId.value})`)
  }
})
</script>
