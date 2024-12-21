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
    <Home isolation-id="tempId" :temp-mode="generateJsonString(basicString)" :title="null" />
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import { generateJsonString } from '@/script/lexer/generateJson'

const basicString = 'and(not(type:"album"), not(tag:"_trashed"))'
</script>
