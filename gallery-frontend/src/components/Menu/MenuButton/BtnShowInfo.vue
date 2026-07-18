<template>
  <v-btn icon="mdi-information-outline" aria-label="Show info" @click="toggle"></v-btn>
  
</template>
<script setup lang="ts">
import { useConstStore } from '@/store/constStore'

const constStore = useConstStore('mainId')

async function toggle() {
  await constStore.updateShowInfo(!constStore.showInfo)
}
</script>
