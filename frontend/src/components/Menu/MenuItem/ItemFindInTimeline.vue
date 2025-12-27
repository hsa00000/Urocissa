<template>
  <v-list-item
    exact
    prepend-icon="mdi-calendar-search-outline"
    :to="`/all?search=&locate=${props.hash}`"
  >
    <v-list-item-title class="wrap">{{ 'Find In Timeline' }}</v-list-item-title>
  </v-list-item>
</template>
<script setup lang="ts">
const props = defineProps<{
  hash: string
}>()
</script>
