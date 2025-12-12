<template>
  <PageTemplate>
    <template #content>
      <EditShareModal
        v-if="modalStore.showEditShareModal && currentEditShareData"
        :edit-share-data="currentEditShareData"
      />
      <ShareDeleteConfirmModal
        v-if="modalStore.showDeleteShareModal && currentDeleteShareData"
        :delete-share-data="currentDeleteShareData"
      />

      <v-container
        v-if="shareStore.fetched"
        id="table-container"
        class="h-100 pa-1 bg-surface-light d-flex align-start"
        fluid
      >
        <v-row justify="center" class="ma-0 w-100">
          <v-col cols="12" sm="12" md="10" lg="8" class="d-flex justify-center">
            <v-card tile flat class="overflow-y-auto w-100">
              <v-data-table
                :headers="headers"
                :items="tableItems"
                :group-by="[{ key: 'albumId' }]"
                item-value="share.url"
                :items-per-page="-1"
                :sort-by="[{ key: 'share.url', order: 'asc' }]"
              >
                <template #[`item.share.url`]="{ item }">
                  {{ item.share.url }}
                </template>

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

                <template #[`item.share.password`]="{ item }">
                  <v-chip
                    size="x-small"
                    :color="item.share.password ? 'red' : 'green'"
                    variant="flat"
                  >
                    {{ item.share.password ? 'Locked' : 'Open' }}
                  </v-chip>
                </template>

                <template #[`item.share.exp`]="{ item }">
                  <span class="text-caption">
                    {{ formatExpiry(item.share.exp) }}
                  </span>
                </template>

                <template #[`item.share.showDownload`]="{ item }">
                  {{ item.share.showDownload }}
                </template>
                <template #[`item.share.showUpload`]="{ item }">
                  {{ item.share.showUpload }}
                </template>
                <template #[`item.share.showMetadata`]="{ item }">
                  {{ item.share.showMetadata }}
                </template>

                <template #[`item.actions`]="{ item }">
                  <div class="d-flex flex-row justify-center ga-1">
                    <v-btn
                      icon="mdi-delete"
                      variant="text"
                      size="small"
                      @click="openDeleteConfirm(item)"
                    />
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
                      :href="`${locationOrigin}/share/${item.albumId}-${item.share.url}`"
                      target="_blank"
                      tag="a"
                    />
                    <v-btn
                      icon="mdi-content-copy"
                      variant="text"
                      size="small"
                      @click="performCopy(item)"
                    />
                  </div>
                </template>

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
                        />
                        <span class="ms-4 font-weight-bold">
                          {{ getGroupDisplayName(item) }}
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
  </PageTemplate>
</template>

<script setup lang="ts">
import { computed, nextTick, ref, onMounted, onBeforeUnmount } from 'vue'
import { useClipboard } from '@vueuse/core'
import EditShareModal from '@/components/Modal/EditShareModal.vue'
import ShareDeleteConfirmModal from '@/components/Modal/ShareDeleteConfirmModal.vue'

import { useInitializedStore } from '@/store/initializedStore'
import { useShareStore } from '@/store/shareStore'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import type { EditShareData } from '@/type/types'
import PageTemplate from './PageLayout/PageTemplate.vue'

// --- Stores ---
const initializedStore = useInitializedStore('mainId')
const shareStore = useShareStore('mainId')
const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')

// --- Utils ---
const locationOrigin = window.location.origin
const { copy } = useClipboard()

// --- State ---
const currentEditShareData = ref<EditShareData | null>(null)
const currentDeleteShareData = ref<EditShareData | null>(null)

// --- Constants ---
const headers = [
  { title: 'Link', key: 'share.url' },
  {
    title: 'Description',
    key: 'share.description',
    width: '200px',
    maxWidth: '200px',
    nowrap: true
  },
  { title: 'Locked', key: 'share.password' },
  { title: 'Expires', key: 'share.exp', width: '180px' },
  { title: 'Allow Download', key: 'share.showDownload' },
  { title: 'Allow Upload', key: 'share.showUpload' },
  { title: 'Show Metadata', key: 'share.showMetadata' },
  { title: 'Actions', key: 'actions', sortable: false }
]

// --- Computed ---
// 這裡負責資料轉換，將 Store 資料轉為 UI 需要的形狀
const tableItems = computed<EditShareData[]>(() => {
  return shareStore.allShares.map((s) => ({
    albumId: s.albumId,
    displayName: s.albumTitle || 'Untitled',
    share: s
  }))
})

// --- Helper Functions (運算邏輯封裝) ---

/**
 * 從 Vuetify 的 Group Item 中提取顯示名稱。
 * 封裝了 .raw 的存取邏輯，避免 Template 變得混亂。
 */
function getGroupDisplayName(groupItem: any): string {
  // 嘗試從群組的第一個項目中獲取原始資料 (raw)
  // 如果結構改變或取不到，則回傳 'Untitled'
  return groupItem.items?.[0]?.raw?.displayName || 'Untitled'
}

/**
 * 格式化過期時間
 */
function formatExpiry(exp: number): string {
  if (exp === 0) return 'Never'
  return new Date(exp * 1000).toLocaleString()
}

// --- Actions ---

function clickEditShare(data: EditShareData) {
  currentEditShareData.value = data
  modalStore.showEditShareModal = true
}

function openDeleteConfirm(data: EditShareData) {
  currentDeleteShareData.value = data
  modalStore.showDeleteShareModal = true
}

async function performCopy(item: EditShareData) {
  await copy(`${locationOrigin}/share/${item.albumId}-${item.share.url}`)
  messageStore.success('Share URL copied to clipboard.')
}

// --- Lifecycle ---

onMounted(async () => {
  // 如果尚未獲取，則執行獲取
  if (!shareStore.fetched) {
    await shareStore.fetchAllShares()
  }

  initializedStore.initialized = true
  await nextTick()

  // 自動展開所有群組
  const groupButtons = Array.from(document.querySelectorAll('button.v-btn')).filter((btn) =>
    btn.querySelector('.mdi-chevron-right')
  ) as HTMLButtonElement[]
  groupButtons.forEach((btn) => btn.click())
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
  background-color: #3d3d3d;
  overflow-y: scroll;
  height: 100dvh;
  width: 100%;
}
</style>
