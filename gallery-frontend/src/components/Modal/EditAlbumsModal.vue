<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditAlbumsModal"
    variant="flat"
    persistent
    id="edit-album-overlay"
  >
    <!-- v‑model 綁字串陣列 -->
    <v-confirm-edit
      v-model="changedAlbumIds"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showEditAlbumsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
          <template #title>Edit&nbsp;Albums</template>

          <template #text>
            <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
              <!-- item-value=albumId (string)，不再 return-object -->
              <v-select
                v-model="proxyModel.value"
                chips
                multiple
                :items="albumItems"
                item-title="displayName"
                item-value="albumId"
                label="Albums"
                closable-chips
              />
            </v-form>
          </template>

          <template #actions>
            <v-spacer />
            <component :is="actions" />
          </template>
        </v-card>
      </template>
    </v-confirm-edit>
  </v-dialog>
</template>

<script setup lang="ts">
/**
 * Modal for editing the album list of one photo (single‑photo view).
 * 改動重點：
 *   • albumId → string
 *   • interface AlbumItem 取代 type
 */
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'

import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import { editAlbumsInWorker } from '@/script/inWorker/editAlbumsInWorker'

import { AlbumInfo } from '@type/types'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'

/* ── state ───────────────────────────────────────────────────────── */
const formIsValid = ref(false)
const changedAlbumIds = ref<string[]>([]) // ← 字串陣列
const submit = ref<(() => void) | undefined>()

/* ── stores & route ──────────────────────────────────────────────── */
const route = useRoute()
const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')

/* ── item 型別 ───────────────────────────────────────────────────── */
interface AlbumItem extends AlbumInfo {
  /** 供 <v-select> chips 顯示用字串 */
  displayName: string
}

/* ── 給 v-select 的 items ───────────────────────────────────────── */
const albumItems = computed<AlbumItem[]>(() => [...albumStore.albums.values()])

/* ── lifecycle ───────────────────────────────────────────────────── */
onMounted(() => {
  const initSubmit = (): (() => void) | undefined => {
    const init = getHashIndexDataFromRoute(route)
    if (!init) {
      console.error('initSubmit: failed to parse route.')
      return
    }

    const { index, data } = init
    if (!data.database) {
      console.error("initSubmit: 'data.database' is undefined.")
      return
    }

    /* 目前照片預設相簿 (string[]) */
    const defaultAlbumIds: string[] = [...data.database.album]
    changedAlbumIds.value = [...defaultAlbumIds]

    const innerSubmit = () => {
      const addAlbumIds = changedAlbumIds.value.filter((id) => !defaultAlbumIds.includes(id))
      const removeAlbumIds = defaultAlbumIds.filter((id) => !changedAlbumIds.value.includes(id))

      editAlbumsInWorker(
        [index], // 目前照片 hashIndex
        addAlbumIds, // string[]
        removeAlbumIds, // string[]
        getIsolationIdByRoute(route)
      )

      modalStore.showEditAlbumsModal = false
    }

    return innerSubmit
  }

  submit.value = initSubmit()
})
</script>
