<template>
  <v-list-item
    prepend-icon="mdi-open-in-new"
    value="view-original-file"
    :href="props.src"
    target="_blank"
  >
    <v-list-item-title class="wrap">{{ 'View Original File' }}</v-list-item-title>
  </v-list-item>
</template>
<script setup lang="ts">
const props = defineProps<{
  src: string
}>()
</script>
