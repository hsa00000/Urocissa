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
    <Home isolation-id="tempId" :filter-determined-by-query="generateJsonString(basicString)">
      <template #reading-bar>
        <SelectionBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import { Album } from '@/script/common/types'
import Home from './Home.vue'
import { generateJsonString } from '@/script/lexer/generateJson'
import SelectionBar from '@/components/NavBar/SelectionBar.vue'
const props = defineProps<{
  album: Album
}>()

const basicString = `and(not(type:"album"), not(tag:"_trashed"), not(album:"${props.album.id}"))`
</script>
