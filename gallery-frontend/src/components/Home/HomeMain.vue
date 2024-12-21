<template>
  <Home isolation-id="mainId" :temp-mode="null" :title="null">
    <template #reading-bar>
      <NavBar />
    </template>
  </Home>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import NavBar from '@/components/NavBar/NavBar.vue'
</script>
