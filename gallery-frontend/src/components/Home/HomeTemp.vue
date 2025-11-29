<template>
  <v-overlay
    :model-value="true"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    transition="false"
    :close-on-back="false"
  >
    <Home isolation-id="tempId" :basic-string="basicString" :search-string="null">
      <template #home-toolbar>
        <HomeTempBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import { Album } from '@type/types'
import Home from './Home.vue'
import HomeTempBar from '@/components/NavBar/HomeBars/HomeTempBar.vue'
const props = defineProps<{
  album: Album
}>()

const basicString = `and(not(type:"album"), not(tag:"_trashed"), not(album:"${props.album.id}"))`
</script>
