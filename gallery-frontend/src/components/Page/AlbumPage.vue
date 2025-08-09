<template>
  <template v-if="album !== undefined">
    <ReadingBar :album="album" />
    <Home isolation-id="mainId" :filter-string="filterString" :search-string="searchString"></Home>
  </template>
</template>

<script setup lang="ts">
import { onBeforeMount, Ref, ref } from 'vue'
import { LocationQueryValue, useRoute } from 'vue-router'
import Home from '@/components/Home/Home.vue'
import ReadingBar from '@/components/NavBar/ReadingBar.vue'
import { useDataStore } from '@/store/dataStore'
import { Album } from '@/type/types'
const props = defineProps<{ filterString: string; albumId: string }>()
const route = useRoute()
const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(undefined)
const dataStore = useDataStore('mainId')
const album: Ref<Album | undefined> = ref(undefined)
onBeforeMount(() => {
  const index = dataStore.hashMapData.get(props.albumId)
  if (index !== undefined) {
    album.value = dataStore.data.get(index)?.album
  }
  searchString.value = route.query.search
})
</script>
