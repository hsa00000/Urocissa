<template>
  <OverlayModal
    v-if="album !== undefined && filterString !== null"
    v-model="modalStore.showHomeIsolatedModal"
    id="home-isolated"
    :close-on-back="true"
    :persistent="false"
    :transition="false"
    overlay-class="d-flex"
  >
    <ReadingBar :album="album" />
    <Home isolation-id="subId" :filter-string="filterString" :search-string="null"> </Home>
  </OverlayModal>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import ReadingBar from '@/components/NavBar/ReadingBar.vue'
import OverlayModal from '@/components/Modal/OverlayModal.vue'
import { Album } from '@type/types'
import { onBeforeMount, Ref, ref } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { useModalStore } from '@/store/modalStore'

const props = defineProps<{
  albumId: string
}>()

const dataStore = useDataStore('mainId')
const modalStore = useModalStore('mainId')
const album: Ref<Album | undefined> = ref(undefined)
const filterString: Ref<string | null> = ref(null)

onBeforeMount(() => {
  const index = dataStore.hashMapData.get(props.albumId)
  if (index !== undefined) {
    album.value = dataStore.data.get(index)?.album
  }
  filterString.value = `and(album:"${props.albumId}", not(tag:"_trashed"))`
})
</script>
