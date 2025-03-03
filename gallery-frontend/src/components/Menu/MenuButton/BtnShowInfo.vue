<template>
  <v-btn icon="mdi-information-outline" @click="infoStore.showInfo = !infoStore.showInfo"></v-btn>
</template>
<script setup lang="ts">
import { useInfoStore } from '@/store/infoStore'

const infoStore = useInfoStore('mainId')
</script>
