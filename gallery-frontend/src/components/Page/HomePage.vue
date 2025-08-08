<template>
  <Home
    isolation-id="mainId"
  :filter-string="filterString"
    :search-string="searchString"
  >
  </Home>
</template>

<script setup lang="ts">
import { onBeforeMount, ref } from 'vue'
import { LocationQueryValue, useRoute } from 'vue-router'
import Home from '../Home/Home.vue'
defineProps<{ filterString: string }>()
const route = useRoute()
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)

onBeforeMount(() => {
  searchString.value = route.query.search
})
</script>
