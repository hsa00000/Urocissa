<template>
  <BaseModal
    v-model="modalStore.showBatchEditTagsModal"
    title="Batch Edit Tags"
    width="500"
    :loading="isSaving"
  >
    <div class="text-body-2 text-medium-emphasis mb-4">
      Modifying tags for {{ selectedCount }} selected items.
    </div>

    <v-form v-model="formIsValid" @submit.prevent="handleSave">
      <v-combobox
        v-model="addedTags"
        label="Add Tags"
        chips
        closable-chips
        multiple
        item-title="tag"
        item-value="tag"
        :items="allTags"
        variant="outlined"
        density="comfortable"
        hide-details="auto"
        class="mb-4"
        :disabled="isSaving"
        autocomplete="off"
        :rules="[rules.noEmpty, rules.noConflictAdded]"
        validate-on="input"
      ></v-combobox>

      <v-combobox
        v-model="removedTags"
        label="Remove Tags"
        chips
        closable-chips
        multiple
        item-title="tag"
        item-value="tag"
        :items="allTags"
        variant="outlined"
        density="comfortable"
        hide-details="auto"
        :disabled="isSaving"
        autocomplete="off"
        :rules="[rules.noEmpty, rules.noConflictRemoved]"
        validate-on="input"
      ></v-combobox>
    </v-form>

    <template #actions>
      <v-spacer />
      <v-btn variant="text" :disabled="isSaving" @click="modalStore.showBatchEditTagsModal = false">
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
import { useTagStore } from '@/store/tagStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useMessageStore } from '@/store/messageStore'
import { editTags } from '@/api/editTags'
import BaseModal from '@/components/Modal/BaseModal.vue'
import { getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()
const modalStore = useModalStore('mainId')
const tagStore = useTagStore('mainId')
const messageStore = useMessageStore('mainId')

const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

// 狀態控制
const formIsValid = ref(false)
const isSaving = ref(false)

// 資料模型
const addedTags = ref<string[]>([])
const removedTags = ref<string[]>([])

// Computed
const allTags = computed(() => tagStore.tags.map((t) => t.tag))
const selectedCount = computed(() => collectionStore.editModeCollection.size)

// 計算是否有任何變更（用於控制 Save 按鈕啟用狀態）
const hasChanges = computed(() => {
  return addedTags.value.length > 0 || removedTags.value.length > 0
})

// === 驗證規則 ===
const rules = {
  // 非空檢查：過濾掉空字串或純空白
  noEmpty: (v: string[]) => {
    const hasEmpty = v.some((tag) => !tag || tag.trim() === '')
    return !hasEmpty || 'Tags cannot be empty.'
  },

  // 互斥檢查 (Added)：檢查是否出現在 Removed 清單中
  noConflictAdded: (v: string[]) => {
    // 找出同時存在於 removedTags 的標籤
    const conflicts = v.filter((tag) => removedTags.value.includes(tag))
    if (conflicts.length > 0) {
      return `Conflict: '${conflicts[0]}' is also in the remove list.`
    }
    return true
  },

  // 互斥檢查 (Removed)：檢查是否出現在 Added 清單中
  noConflictRemoved: (v: string[]) => {
    // 找出同時存在於 addedTags 的標籤
    const conflicts = v.filter((tag) => addedTags.value.includes(tag))
    if (conflicts.length > 0) {
      return `Conflict: '${conflicts[0]}' is also in the add list.`
    }
    return true
  }
}

// 初始化數據
const initializeData = () => {
  addedTags.value = []
  removedTags.value = []
  formIsValid.value = true // 重置時預設為 true (因為空陣列是合法的起始狀態，直到使用者輸入錯誤)
}

// 監聽 Modal 開啟
watch(
  () => modalStore.showBatchEditTagsModal,
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  }
)

const handleSave = async () => {
  // 雙重保險：如果無變更或表單驗證未通過，則不執行
  if (!hasChanges.value || !formIsValid.value) return

  isSaving.value = true
  try {
    const selectedHashes = Array.from(collectionStore.editModeCollection)

    await editTags(selectedHashes, addedTags.value, removedTags.value, isolationId)

    messageStore.success('Batch update tags successful.')
    modalStore.showBatchEditTagsModal = false

    // 清空選取狀態
    collectionStore.leaveEdit()
  } catch (e) {
    console.error(e)
    messageStore.error('Batch update tags failed.')
  } finally {
    isSaving.value = false
  }
}
</script>
