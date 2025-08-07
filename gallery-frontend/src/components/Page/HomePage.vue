<template>
  <Home
    isolation-id="mainId"
    :filter-string="filterStringStore.filterString"
    :search-string="searchString"
  >
    <template #reading-bar>
      <NavBar />
    </template>
  </Home>
</template>

<script setup lang="ts">
import { useFilterStringStore } from '@/store/filterStringStore'
import { onBeforeMount, ref } from 'vue'
import { LocationQueryValue, useRoute } from 'vue-router'
import Home from '../Home/Home.vue'
import NavBar from '@/components/NavBar/NavBar.vue'
const filterStringStore = useFilterStringStore()
const route = useRoute()
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)

onBeforeMount(() => {
  searchString.value = route.query.search
})
</script>
