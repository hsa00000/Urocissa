<template>
  <NavBar />
  <EditShareModal
    v-if="modalStore.showShareModal && currentEditShareData !== undefined"
    :edit-share-data="currentEditShareData"
  />
  <v-container
    v-if="albumStore.fetched"
    id="table-container"
    class="pa-1 bg-grey-darken-3 d-flex align-start"
    :style="{ height: `calc(100% - ${navBarHeight}px)` }"
    fluid
  >
    <v-row justify="center" class="ma-0">
      <v-col cols="12" sm="12" md="10" lg="8" class="d-flex justify-center">
        <v-card tile flat class="overflow-y-auto w-100">
          <v-data-table
            :headers="headers"
            :items="tableItems"
            :group-by="[{ key: 'displayName' }]"
            item-value="url"
            :items-per-page="-1"
          >
            <template #group-header="{ item, columns, toggleGroup, isGroupOpen }">
              <tr>
                <td :colspan="columns.length">
                  <div class="d-flex align-center">
                    <v-btn
                      :icon="isGroupOpen(item) ? '$expand' : '$next'"
                      color="medium-emphasis"
                      density="comfortable"
                      size="small"
                      variant="outlined"
                      @click="toggleGroup(item)"
                    ></v-btn>
                    <span class="ms-4 font-weight-bold"> Album: {{ item.value }} </span>
                  </div>
                </td>
              </tr>
            </template>
            <template #[`item.actions`]="{ item }">
              <div class="d-flex flex-row justify-center ga-1">
                <v-btn icon="mdi-pencil" variant="text" @click="clickEditShare(item)" />
              </div>
            </template>
          </v-data-table>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch, onMounted, onBeforeUnmount } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { navBarHeight } from '@/type/constants'
import NavBar from '@/components/NavBar/NavBar.vue'
import { useAlbumStore } from '@/store/albumStore'
import { useModalStore } from '@/store/modalStore'
import { EditShareData } from '@/type/types'
import EditShareModal from '@/components/Modal/EditShareModal.vue'

const initializedStore = useInitializedStore('mainId')
const albumStore = useAlbumStore('mainId')
const modalStore = useModalStore('mainId')
const dynamicWidth = ref<number>(0)
const tableRef = ref<HTMLElement | null>(null)

const updateDynamicWidth = () => {
  const tableWidth = tableRef.value?.offsetWidth ?? 0
  dynamicWidth.value = tableWidth <= 300 ? 300 : tableWidth
}

// Single reactive variable for storing edit share data (of type EditShareData)
const currentEditShareData = ref<EditShareData | undefined>(undefined)

const headers = [
  { title: 'Link', key: 'share.url' },
  { title: 'Description', key: 'share.description' },
  { title: 'Edit', key: 'actions', align: 'center' as const, sortable: false }
]

// Compute table items using EditShareData as the type.
const tableItems = computed<EditShareData[]>(() => {
  const result: EditShareData[] = []

  for (const album of albumStore.albums.values()) {
    for (const [, share] of album.shareList) {
      result.push({
        albumId: album.albumId,
        displayName: album.displayName,
        share: share
      })
    }
  }

  return result
})

function clickEditShare(data: EditShareData) {
  currentEditShareData.value = data
  modalStore.showShareModal = true
}

watch(
  () => initializedStore.initialized,
  () => {
    if (initializedStore.initialized) {
      updateDynamicWidth()
    }
  }
)

onMounted(async () => {
  if (!albumStore.fetched) {
    await albumStore.fetchAlbums()
  }
  initializedStore.initialized = true

  await nextTick()

  // Find all buttons with mdi-chevron-right (for unexpanded groups)
  const groupButtons = Array.from(document.querySelectorAll('button.v-btn')).filter((btn) =>
    btn.querySelector('.mdi-chevron-right')
  ) as HTMLButtonElement[]

  for (const btn of groupButtons) {
    btn.click()
  }
})
onBeforeUnmount(() => {
  initializedStore.initialized = false
})
</script>

<style scoped>
#table-container {
  display: flex;
  justify-content: center;
  position: relative;
  padding: 4px;
  padding-top: 4px;
  background-color: #3d3d3d;
  overflow-y: scroll;
  height: 100dvh;
  width: 100%;
}
</style>
