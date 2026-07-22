<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, shallowRef, watch } from 'vue'
import { useRoute, useRouter } from 'vue-router'
import { useAlbumStore } from '@/store/albumStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useSearchFacetStore } from '@/store/searchFacetStore'
import { IMAGE_EXTENSIONS, VIDEO_EXTENSIONS, useUploadStore } from '@/store/uploadStore'
import PageTemplate from './PageLayout/PageTemplate.vue'
import UploadQueuePanel from './UploadPage/UploadQueuePanel.vue'

const route = useRoute()
const router = useRouter()
const albumStore = useAlbumStore('mainId')
const initializedStore = useInitializedStore('mainId')
const searchFacetStore = useSearchFacetStore()
const uploadStore = useUploadStore('mainId')

const showFormats = shallowRef(false)
const albumLoadComplete = shallowRef(albumStore.fetched)

const albums = computed(() => Array.from(albumStore.albums.values()))
const tagSuggestions = computed(() => searchFacetStore.tags.map((tag) => tag.value))
const presignedAlbumIds = computed<string[]>({
  get: () => uploadStore.presignAlbumIds,
  set: (albumIds) => {
    uploadStore.presignAlbumIds = [...albumIds]
  }
})
const presignedTags = computed<string[]>({
  get: () => uploadStore.presignTags,
  set: (tags) => {
    uploadStore.presignTags = [...tags]
  }
})
const routeAlbumIds = computed(() => {
  const raw = route.query.albumId
  const values = Array.isArray(raw) ? raw : [raw]
  return [...new Set(values.filter((value): value is string => typeof value === 'string' && value !== ''))]
})

async function loadAlbums(): Promise<void> {
  if (albumStore.fetched) {
    albumLoadComplete.value = true
    return
  }

  try {
    await albumStore.fetchAlbums()
  } finally {
    albumLoadComplete.value = true
  }
}

async function loadTags(): Promise<void> {
  if (!searchFacetStore.fetched) await searchFacetStore.fetchFacets()
}

function sameValues(left: readonly string[], right: readonly string[]): boolean {
  return left.length === right.length && left.every((value, index) => value === right[index])
}

function replaceAlbumQuery(albumIds: readonly string[]): void {
  const query = { ...route.query }
  if (albumIds.length === 0) delete query.albumId
  else {
    const firstAlbumId = albumIds[0]
    query.albumId = albumIds.length === 1 && firstAlbumId !== undefined
      ? firstAlbumId
      : [...albumIds]
  }
  void router.replace({ path: route.path, query })
}

watch(
  [routeAlbumIds, () => albumStore.albums.size, albumLoadComplete],
  ([albumIds]) => {
    if (!albumLoadComplete.value) return

    const validAlbumIds = albumIds.filter((albumId) => albumStore.albums.has(albumId))
    if (!sameValues(presignedAlbumIds.value, validAlbumIds)) {
      presignedAlbumIds.value = validAlbumIds
    }
    if (!sameValues(albumIds, validAlbumIds)) replaceAlbumQuery(validAlbumIds)
  },
  { immediate: true }
)

watch(presignedAlbumIds, (albumIds) => {
  if (sameValues(albumIds, routeAlbumIds.value)) return
  replaceAlbumQuery(albumIds)
})

onMounted(async () => {
  await Promise.allSettled([loadAlbums(), loadTags()])
  initializedStore.initialized = true
})

onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>

<template>
  <PageTemplate
    preset="card"
    width="wide"
    :fill-height="true"
    card-class="upload-page-card overflow-hidden w-100 rounded-lg bg-surface"
  >
    <template #content>
      <div class="upload-page d-flex flex-column h-100">
        <UploadQueuePanel
          v-model:presigned-album-ids="presignedAlbumIds"
          v-model:presigned-tags="presignedTags"
          :albums="albums"
          :tag-suggestions="tagSuggestions"
          @show-formats="showFormats = true"
        />
      </div>

      <v-dialog v-model="showFormats" max-width="520">
        <v-card>
          <v-card-title>Supported File Formats</v-card-title>
          <v-card-text>
            <div class="text-body-2 font-weight-bold mb-2">Images</div>
            <div class="d-flex flex-wrap ga-2 mb-5">
              <v-chip v-for="extension in IMAGE_EXTENSIONS" :key="extension" size="small" variant="tonal">
                .{{ extension }}
              </v-chip>
            </div>

            <div class="text-body-2 font-weight-bold mb-2">Videos</div>
            <div class="d-flex flex-wrap ga-2">
              <v-chip v-for="extension in VIDEO_EXTENSIONS" :key="extension" size="small" variant="tonal">
                .{{ extension }}
              </v-chip>
            </div>
          </v-card-text>
          <v-card-actions>
            <v-spacer />
            <v-btn color="primary" variant="text" @click="showFormats = false">Close</v-btn>
          </v-card-actions>
        </v-card>
      </v-dialog>
    </template>
  </PageTemplate>
</template>

<style scoped>
.upload-page {
  min-height: 0;
}
</style>
