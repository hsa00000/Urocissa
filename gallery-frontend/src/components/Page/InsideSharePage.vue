<template>
  <HomePage />
</template>

<script setup lang="ts">
import HomePage from '@/components/Home/Page/HomeMain.vue'
defineProps<{ id: string; hash?: string }>()
</script>
