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
    <Home isolation-id="shareId" :temp-mode="generateJsonString(basicString)"> </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import { generateJsonString } from '@/script/lexer/generateJson'
const props = defineProps<{
  albumId: string
  shareId: string
}>()

const basicString = `and(not(tag:"_trashed"), album:"${props.albumId}")`
</script>
