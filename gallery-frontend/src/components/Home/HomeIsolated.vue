<script setup lang="ts">
import { computed } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import Home from './Home.vue'
import HomeIsolatedBar from '@/components/NavBar/HomeBars/HomeIsolatedBar.vue'
import { useResolvedRouteResource } from '@/script/hook/useRouteResource'
import { useCollectionStore } from '@/store/collectionStore'

const route = useRoute()
const router = useRouter()
const collectionStore = useCollectionStore('subId')

const hash = computed(() =>
  typeof route.params.hash === 'string' ? route.params.hash : ''
)
const basicString = computed(() => `and(album:"${hash.value}", trashed:false)`)
const searchString = computed(() => route.query.subSearch)

const { resource, status, errorMessage, retry } = useResolvedRouteResource(
  hash,
  'mainId',
  'detailId',
  ['album']
)

const album = computed(() =>
  resource.value?.data.type === 'album' ? resource.value.data : undefined
)

const overlayVisible = computed<boolean>({
  get: () => true,
  set: (value) => {
    if (value) return
    if (collectionStore.editModeOn) collectionStore.editModeOn = false
    else router.back()
  }
})

// A query-only reader update refreshes Home's snapshot in place. Only a new
// parent resource replaces the collection component.
const isolatedHomeKey = computed(() => `isolated-${hash.value}`)

const errorTitle = computed(() => {
  switch (status.value) {
    case 'invalid':
      return 'Invalid item ID'
    case 'not-found':
      return 'Item not found'
    case 'wrong-type':
      return 'This item is not an album'
    default:
      return 'Unable to load album'
  }
})

const errorBody = computed(() => {
  if (status.value === 'wrong-type') {
    return 'The reader route requires its parent item to be an album.'
  }
  return errorMessage.value ?? 'The album could not be loaded. Please try again.'
})

function leave(): void {
  router.back()
}

function retryLoad(): void {
  void retry()
}
</script>

<template>
  <v-overlay
    id="read-page"
    v-model="overlayVisible"
    height="100%"
    width="100%"
    content-class="w-100 h-100 position-relative"
    class="d-flex"
    :transition="false"
    :close-on-back="false"
  >
    <!-- Home mounts immediately so contents and the level-4 route load independently. -->
    <Home
      :key="isolatedHomeKey"
      isolation-id="subId"
      :basic-string="basicString"
      :search-string="searchString"
    >
      <template v-if="album !== undefined" #home-toolbar>
        <HomeIsolatedBar :album="album" />
      </template>
    </Home>

    <v-sheet
      v-if="status === 'loading' || status === 'idle'"
      color="background"
      class="position-absolute h-100 w-100 d-flex flex-column align-center justify-center ga-4"
      style="inset: 0"
    >
      <v-progress-circular indeterminate color="primary" size="64" />
      <span class="text-body-1">Loading album…</span>
    </v-sheet>

    <v-sheet
      v-else-if="status !== 'ready'"
      color="background"
      class="position-absolute h-100 w-100 d-flex align-center justify-center pa-4"
      style="inset: 0"
    >
      <v-alert
        type="error"
        variant="tonal"
        icon="mdi-alert-circle-outline"
        :title="errorTitle"
        max-width="640"
        class="w-100"
      >
        <p class="mb-2">{{ errorBody }}</p>
        <p class="text-body-2 text-break mb-4">Requested ID: {{ hash }}</p>
        <v-btn
          v-if="status !== 'wrong-type'"
          color="primary"
          variant="flat"
          class="me-2"
          @click="retryLoad"
        >
          Retry
        </v-btn>
        <v-btn variant="text" @click="leave">Leave</v-btn>
      </v-alert>
    </v-sheet>
  </v-overlay>
</template>
