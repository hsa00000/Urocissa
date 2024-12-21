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
    <Home isolation-id="subId" :temp-mode="null" :title="props.title" />
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'

const props = defineProps<{
  title: string
}>()
</script>
