<template>
  <Home
    class="w-100 h-100"
    isolation-id="mainId"
    :basic-string="props.basicString"
    :search-string="searchString"
  />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import Home from './Home.vue'
import { useRoute } from 'vue-router'

const route = useRoute()
const searchString = computed(() => route.query.search)

const props = defineProps<{
  basicString: string | null
}>()

</script>
