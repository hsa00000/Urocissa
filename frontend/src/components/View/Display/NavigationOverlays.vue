<template>
  <div>
    <v-card
      width="100"
      v-if="show && previousHash !== undefined"
      color="transparent"
      class="navigate-left d-flex align-center justify-center h-50"
      style="position: absolute; left: 0; top: 50%; transform: translateY(-50%); z-index: 1"
      :to="previousPage"
      replace
    >
      <v-icon>mdi-arrow-left</v-icon>
    </v-card>
    <v-card
      width="100"
      v-if="show && nextHash !== undefined"
      color="transparent"
      class="navigate-right d-flex align-center justify-center h-50"
      style="position: absolute; right: 0; top: 50%; transform: translateY(-50%); z-index: 1"
      :to="nextPage"
      replace
    >
      <v-icon>mdi-arrow-right</v-icon>
    </v-card>
  </div>
</template>

<script setup lang="ts">
import type { RouteLocationRaw } from 'vue-router'
defineProps<{
  previousHash: string | undefined
  nextHash: string | undefined
  previousPage: RouteLocationRaw | undefined
  nextPage: RouteLocationRaw | undefined
  show: boolean
}>()
</script>
