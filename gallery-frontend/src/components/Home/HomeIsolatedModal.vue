<template>
  <OverlayModal
    v-model="open"
    id="home-isolated"
    :close-on-back="true"
    :persistent="false"
    :transition="false"
    overlay-class="d-flex"
  >
    <Home
      v-if="album !== undefined && filterString !== null"
      isolation-id="subId"
      :filter-string="filterString"
      :search-string="null"
    >
      <template #reading-bar>
        <ReadingBar :album="album" />
      </template>
    </Home>
  </OverlayModal>
  
</template>
<script setup lang="ts">
import Home from './Home.vue'
import ReadingBar from '@/components/NavBar/ReadingBar.vue'
import OverlayModal from '@/components/Modal/OverlayModal.vue'
import { Album } from '@type/types'
import { onBeforeMount, Ref, ref, watch } from 'vue'
import { useDataStore } from '@/store/dataStore'

const props = defineProps<{
  albumId: string
  modelValue: boolean
}>()

const emit = defineEmits<(e: 'update:modelValue', v: boolean) => void>()

const open = ref(props.modelValue)
watch(
  () => props.modelValue,
  v => (open.value = v)
)
watch(open, v => {
  emit('update:modelValue', v)
})

const dataStore = useDataStore('mainId')
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
