<template>
  <v-btn icon="mdi mdi-arrow-left" @click="router.back()"></v-btn>
</template>
<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>
