<template>
  <v-overlay
    v-model="overlayVisible"
    :height="'100%'"
    :width="'100%'"
    class="d-flex"
    id="view-page"
    :transition="false"
    :close-on-back="false"
  >
    <Home
      v-if="album !== undefined && basicString !== null"
      :key="isolatedHomeKey"
      isolation-id="subId"
      :basic-string="basicString"
      :search-string="searchString"
    >
      <template #home-toolbar>
        <HomeIsolatedBar :album="album" />
      </template>
    </Home>
  </v-overlay>
</template>
<script setup lang="ts">
import Home from './Home.vue'
import HomeIsolatedBar from '@/components/NavBar/HomeBars/HomeIsolatedBar.vue'
import { GalleryAlbum } from '@type/types'
import { computed, onBeforeMount, Ref, ref, watch } from 'vue'
import { useCollectionStore } from '@/store/collectionStore'
import { LocationQueryValue, useRoute, useRouter } from 'vue-router'
import { useDataStore } from '@/store/dataStore'

const route = useRoute()
const router = useRouter()
const dataStore = useDataStore('mainId')
const album: Ref<GalleryAlbum | undefined> = ref(undefined)
const basicString: Ref<string | null> = ref(null)

const searchString = ref<LocationQueryValue | LocationQueryValue[] | undefined>(null)
const collectionStore = useCollectionStore('subId')

const overlayVisible = computed<boolean>({
  get() {
    return true
  },
  set(val: boolean) {
    if (!val) {
      if (collectionStore.editModeOn) {
        collectionStore.editModeOn = false
      } else {
        router.back()
      }
    }
  }
})

const subSearchKey = computed(() => {
  const v = route.query.subSearch
  if (typeof v === 'string') return v
  if (Array.isArray(v)) return v.join(',')
  return ''
})

const locateKey = computed(() => {
  const v = route.query.locate
  if (typeof v === 'string') return v
  if (Array.isArray(v)) return v.join(',')
  return ''
})

const priorityKey = computed(() => {
  const v = route.query.priority_id
  if (typeof v === 'string') return v
  if (Array.isArray(v)) return v.join(',')
  return ''
})

const sortKey = computed(() => {
  const v = route.query.sort
  if (typeof v === 'string') return v
  if (Array.isArray(v)) return v.join(',')
  return ''
})

const hashKey = computed(() => (typeof route.params.hash === 'string' ? route.params.hash : ''))

// This forces ONLY the isolated Home to remount when subSearch changes
const isolatedHomeKey = computed(() => {
  return `isolated-${hashKey.value}-${subSearchKey.value}-${locateKey.value}-${priorityKey.value}-${sortKey.value}`
})

watch(
  () => route.query.subSearch,
  (v) => {
    searchString.value = v
  },
  { immediate: true }
)

onBeforeMount(() => {
  const hash = route.params.hash
  if (typeof hash === 'string') {
    const index = dataStore.hashMapData.get(hash)
    if (index !== undefined) {
      const data = dataStore.data.get(index)
      if (data?.type === 'album') {
        album.value = data
      }
    }
  }

  const album_id = route.params.hash
  if (typeof album_id === 'string') {
    basicString.value = `and(album:"${album_id}", trashed:false)`
  }
})
</script>
