<template>
  <BaseModal
    v-model="modalStore.showBatchEditAlbumsModal"
    title="Batch Edit Albums"
    width="500"
    :loading="isSaving"
  >
    <div class="text-body-2 text-medium-emphasis mb-4">
      Modifying albums for {{ selectedCount }} selected items.
    </div>

    <v-form v-model="formIsValid" @submit.prevent="handleSave">
      <v-combobox
        v-model="addedAlbums"
        label="Add to Albums"
        chips
        closable-chips
        multiple
        item-title="title"
        item-value="id"
        :items="albumList"
        variant="outlined"
        density="comfortable"
        hide-details="auto"
        class="mb-4"
        :disabled="isSaving"
        :return-object="false"
      ></v-combobox>

      <v-combobox
        v-model="removedAlbums"
        label="Remove from Albums"
        chips
        closable-chips
        multiple
        item-title="title"
        item-value="id"
        :items="albumList"
        variant="outlined"
        density="comfortable"
        hide-details="auto"
        :disabled="isSaving"
        :return-object="false"
      ></v-combobox>
    </v-form>

    <template #actions>
      <v-spacer />
      <v-btn
        variant="text"
        :disabled="isSaving"
        @click="modalStore.showBatchEditAlbumsModal = false"
      >
        Cancel
      </v-btn>
      <v-btn
        color="primary"
        variant="flat"
        :loading="isSaving"
        :disabled="!formIsValid"
        @click="handleSave"
      >
        Save
      </v-btn>
    </template>
  </BaseModal>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useAlbumStore } from '@/store/albumStore'
import { useCollectionStore } from '@/store/collectionStore' // 使用 CollectionStore
import { useMessageStore } from '@/store/messageStore'
import { editAlbums } from '@/api/editAlbums' // 使用標準 API
import BaseModal from '@/components/Modal/BaseModal.vue'
import { getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()
const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const messageStore = useMessageStore('mainId')

// 取得 isolationId 以初始化 collectionStore
const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

const formIsValid = ref(false)
const isSaving = ref(false)

const addedAlbums = ref<string[]>([])
const removedAlbums = ref<string[]>([])

const selectedCount = computed(() => collectionStore.editModeCollection.size)

// 轉換相簿列表格式 (Map -> Array)
const albumList = computed(() =>
  Array.from(albumStore.albums.values()).map((a) => ({
    title: a.displayName || a.albumId,
    id: a.albumId
  }))
)

// 初始化：每次開啟時清空輸入框
const initializeData = () => {
  addedAlbums.value = []
  removedAlbums.value = []
}

watch(
  () => modalStore.showBatchEditAlbumsModal, // 修正拼字
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  }
)

const handleSave = async () => {
  if (addedAlbums.value.length === 0 && removedAlbums.value.length === 0) {
    modalStore.showBatchEditAlbumsModal = false
    return
  }

  isSaving.value = true
  try {
    // 從 CollectionStore 取得選取的 hashes (Set -> Array)
    const selectedHashes = Array.from(collectionStore.editModeCollection)

    await editAlbums(selectedHashes, addedAlbums.value, removedAlbums.value, isolationId)

    messageStore.success('Batch update albums successful.')
    modalStore.showBatchEditAlbumsModal = false
    collectionStore.editModeOn = false // 關閉編輯模式 (相當於 clearSelected)
  } catch (e) {
    console.error(e)
    messageStore.error('Batch update albums failed.')
  } finally {
    isSaving.value = false
  }
}
</script>
