<template>
  <NavBar />
  <EditShareModal
    v-if="modalStore.showEditShareModal && currentEditShareData !== undefined"
    :edit-share-data="currentEditShareData"
  />
  <v-container
    v-if="albumStore.fetched"
    id="table-container"
    class="pa-1 bg-grey-darken-3 d-flex align-start"
    :style="{ height: `calc(100% - ${navBarHeight}px)` }"
    fluid
  >
    <v-row justify="center" class="ma-0 w-100">
      <v-col cols="12" sm="12" md="10" lg="8" class="d-flex justify-center">
        <v-card tile flat class="overflow-y-auto w-100">
          <v-data-table
            :headers="headers"
            :items="tableItems"
            :group-by="[{ key: 'albumId' }]"
            item-value="url"
            :items-per-page="-1"
            :sort-by="[{ key: 'share.url', order: 'asc' }]"
          >
            <!-- 純顯示網址 -->
            <template #[`item.share.url`]="{ item }">
              {{ item.share.url }}
            </template>

            <!-- 描述加 Tooltip -->
            <template #[`item.share.description`]="{ item }">
              <v-tooltip location="top" :open-on-click="true">
                <template #activator="{ props }">
                  <span v-bind="props" class="text-truncate">
                    {{ item.share.description }}
                  </span>
                </template>
                <span>{{ item.share.description }}</span>
              </v-tooltip>
            </template>

            <!-- 所有操作按鈕集中這裡 -->
            <template #[`item.actions`]="{ item }">
              <div class="d-flex flex-row justify-center ga-1">
                <v-btn
                  icon="mdi-pencil"
                  variant="text"
                  size="small"
                  @click="clickEditShare(item)"
                />
                <v-btn
                  icon="mdi-open-in-new"
                  variant="text"
                  size="small"
                  :href="`${locationOrigin}/share-${item.albumId}-${item.share.url}`"
                  target="_blank"
                  tag="a"
                />
                <v-btn
                  icon="mdi-content-copy"
                  variant="text"
                  size="small"
                  @click="copy(`${locationOrigin}/share-${item.albumId}-${item.share.url}`)"
                />
              </div>
            </template>

            <!-- 群組標頭 -->
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
                    <span class="ms-4 font-weight-bold">
                      {{ albumStore.albums.get(item.value)?.displayName }}
                    </span>
                    <v-btn
                      icon="mdi-open-in-new"
                      variant="text"
                      size="small"
                      class="ms-2"
                      :href="`${locationOrigin}/albums/view/${item.value}/read`"
                      target="_blank"
                      tag="a"
                    />
                  </div>
                </td>
              </tr>
            </template>
          </v-data-table>
        </v-card>
      </v-col>
    </v-row>
  </v-container>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, watch, onMounted, onBeforeUnmount, watchEffect } from 'vue'
import { useInitializedStore } from '@/store/initializedStore'
import { navBarHeight } from '@/type/constants'
import NavBar from '@/components/NavBar/NavBar.vue'
import { useAlbumStore } from '@/store/albumStore'
import { useModalStore } from '@/store/modalStore'
import { EditShareData } from '@/type/types'
import EditShareModal from '@/components/Modal/EditShareModal.vue'
import { useClipboard } from '@vueuse/core'

const initializedStore = useInitializedStore('mainId')
const albumStore = useAlbumStore('mainId')
const modalStore = useModalStore('mainId')
const locationOrigin = window.location.origin

const { copy } = useClipboard()

const currentEditShareData = ref<EditShareData | undefined>(undefined)

const headers = [
  { title: 'Link', key: 'share.url' },
  {
    title: 'Description',
    key: 'share.description',
    width: '200px',
    maxWidth: '200px',
    nowrap: true
  },
  {
    title: 'Actions',
    key: 'actions',
    align: 'center' as const,
    sortable: false
  }
]

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
  modalStore.showEditShareModal = true
}

watchEffect(() => {
  console.log('modalStore.showEditShareModal is', modalStore.showEditShareModal)
  console.log('currentEditShareData is', currentEditShareData.value)
})

watch(
  () => initializedStore.initialized,
  () => {
    if (initializedStore.initialized) {
      // layout update hook
    }
  }
)

onMounted(async () => {
  if (!albumStore.fetched) {
    await albumStore.fetchAlbums()
  }
  initializedStore.initialized = true
  await nextTick()

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
