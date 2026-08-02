<template>
  <v-btn icon="mdi mdi-arrow-left" aria-label="Leave current view" @click="router.back()" />
</template>
<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>
