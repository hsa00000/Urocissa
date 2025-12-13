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
        autocomplete="off"
        :rules="[rules.noEmpty, rules.noConflictAdded]"
        validate-on="input"
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
        autocomplete="off"
        :rules="[rules.noEmpty, rules.noConflictRemoved]"
        validate-on="input"
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
        :disabled="!hasChanges || !formIsValid"
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
import { useCollectionStore } from '@/store/collectionStore'
import { useMessageStore } from '@/store/messageStore'
import { editAlbums } from '@/api/editAlbums'
import BaseModal from '@/components/Modal/BaseModal.vue'
import { getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()
const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const messageStore = useMessageStore('mainId')

// 取得 isolationId 以初始化 collectionStore
const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

// 狀態控制
const formIsValid = ref(false)
const isSaving = ref(false)

// 資料模型 (這裡存的是 ID 字串陣列)
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

// 計算是否有任何變更
const hasChanges = computed(() => {
  return addedAlbums.value.length > 0 || removedAlbums.value.length > 0
})

// === 輔助函式：透過 ID 找相簿名稱（用於顯示錯誤訊息） ===
const getAlbumName = (id: string) => {
  const target = albumList.value.find((a) => a.id === id)
  return target ? target.title : id
}

// === 驗證規則 ===
const rules = {
  // 非空檢查
  noEmpty: (v: string[]) => {
    const hasEmpty = v.some((id) => !id || (typeof id === 'string' && id.trim() === ''))
    return !hasEmpty || 'Selection cannot be empty.'
  },

  // 互斥檢查 (Added)：檢查是否出現在 Removed 清單中
  noConflictAdded: (ids: string[]) => {
    // 找出同時存在於 removedAlbums 的 ID
    const conflictId = ids.find((id) => removedAlbums.value.includes(id))

    if (conflictId) {
      const name = getAlbumName(conflictId)
      return `Conflict: Album '${name}' is also in the remove list.`
    }
    return true
  },

  // 互斥檢查 (Removed)：檢查是否出現在 Added 清單中
  noConflictRemoved: (ids: string[]) => {
    // 找出同時存在於 addedAlbums 的 ID
    const conflictId = ids.find((id) => addedAlbums.value.includes(id))

    if (conflictId) {
      const name = getAlbumName(conflictId)
      return `Conflict: Album '${name}' is also in the add list.`
    }
    return true
  }
}

// 初始化：每次開啟時清空輸入框並重置驗證狀態
const initializeData = () => {
  addedAlbums.value = []
  removedAlbums.value = []
  formIsValid.value = true // 初始狀態視為有效
}

watch(
  () => modalStore.showBatchEditAlbumsModal,
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  }
)

const handleSave = async () => {
  // 雙重保險
  if (!hasChanges.value || !formIsValid.value) return

  isSaving.value = true
  try {
    const selectedHashes = Array.from(collectionStore.editModeCollection)

    await editAlbums(selectedHashes, addedAlbums.value, removedAlbums.value, isolationId)

    messageStore.success('Batch update albums successful.')
    modalStore.showBatchEditAlbumsModal = false
    collectionStore.leaveEdit()
  } catch (e) {
    console.error(e)
    messageStore.error('Batch update albums failed.')
  } finally {
    isSaving.value = false
  }
}
</script>
