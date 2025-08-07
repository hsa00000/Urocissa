<template>
  <Home isolation-id="mainId" :basic-string="props.basicString" :search-string="searchString">
    <template #reading-bar>
      <ShareBar />
    </template>
  </Home>
</template>
<script setup lang="ts">
import { onBeforeMount, ref } from 'vue'
import Home from './Home.vue'
import ShareBar from '@/components/NavBar/ShareBar.vue'
import { LocationQueryValue, useRoute } from 'vue-router'

const route = useRoute()
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)

const props = defineProps<{
  basicString: string | null
}>()

onBeforeMount(() => {
  searchString.value = route.query.search
})
</script>
