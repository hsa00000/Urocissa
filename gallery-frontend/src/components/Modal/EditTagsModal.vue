<template>
  <BaseModal
    v-if="submit !== undefined"
    v-model="modalStore.showEditTagsModal"
    title="Edit Tags"
    width="400"
    id="edit-tag-overlay"
    :loading="isSaving"
  >
    <v-form v-model="formIsValid" @submit.prevent="handleSave">
      <v-combobox
        v-model="editingTags"
        chips
        multiple
        item-title="tag"
        item-value="tag"
        :items="filteredTagList"
        label="Tags"
        closable-chips
        variant="outlined"
        autocomplete="off"
        hide-details
        :disabled="isSaving"
      />
    </v-form>

    <template #actions>
      <v-spacer />
      <v-btn variant="text" :disabled="isSaving" @click="modalStore.showEditTagsModal = false">
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
/**
 * This modal is used for editing the tags of a single photo on the single photo view page.
 */
import { ref, computed, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useTagStore } from '@/store/tagStore'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'
import { editTags } from '@/api/editTags'
import BaseModal from '@/components/Modal/BaseModal.vue'

const formIsValid = ref(false)
const changedTagsArray = ref<string[]>([]) // 這裡存放的是"當前最新的"標籤狀態 (在 save 時會被更新)
const editingTags = ref<string[]>([]) // 這裡存放 UI 編輯中的狀態
const submit = ref<(() => Promise<void>) | undefined>(undefined)
const isSaving = ref(false)

const route = useRoute()
const modalStore = useModalStore('mainId')
const tagStore = useTagStore('mainId')

const tagList = computed(() => tagStore.tags)
const filteredTagList = computed(() =>
  tagList.value.filter((tag) => !specialTag(tag.tag)).map((tag) => tag.tag)
)

const specialTag = (tag: string): boolean => {
  return tag === '_archived' || tag === '_favorite' || tag === '_trashed'
}

// 將初始化邏輯提取為函數
const initializeData = () => {
  const initializeResult = getHashIndexDataFromRoute(route)

  if (initializeResult === undefined) {
    // 可能是路由還沒準備好，或是資料還沒 fetch 到
    console.warn('EditTagsModal: Failed to initialize result from route.')
    submit.value = undefined
    return
  }

  const { index, data } = initializeResult
  const defaultTags: string[] = data.data.tags // 這是 DB 中的原始 tags

  // 初始化 changedTagsArray (視為當前 DB 狀態)
  changedTagsArray.value = defaultTags.filter((tag) => !specialTag(tag))

  // 初始化 UI 顯示的 editingTags (複製一份)
  editingTags.value = [...changedTagsArray.value]

  // 定義 submit 函數 (這裡的 defaultTags 會被閉包捕獲，代表"修改前"的狀態)
  const innerSubmit = async () => {
    const hashArray: number[] = [index]

    // 計算新增的標籤：存在於 changedTagsArray 但不在 defaultTags 中
    const addTagsArrayComputed = changedTagsArray.value.filter(
      (tag) => !specialTag(tag) && !defaultTags.includes(tag)
    )

    // 計算移除的標籤：存在於 defaultTags 但不在 changedTagsArray 中
    const removeTagsArrayComputed = defaultTags.filter(
      (tag) => !specialTag(tag) && !changedTagsArray.value.includes(tag)
    )

    const isolationId = getIsolationIdByRoute(route)

    await editTags(hashArray, addTagsArrayComputed, removeTagsArrayComputed, isolationId)
    modalStore.showEditTagsModal = false
  }

  submit.value = innerSubmit
}

// 監聽 Modal 開啟事件
watch(
  () => modalStore.showEditTagsModal,
  (isOpen) => {
    if (isOpen) {
      // 每次開啟時，重新初始化數據
      initializeData()
    }
  },
  { immediate: true } // 如果組件掛載時 Modal 已經是開啟狀態(極少見)，也能觸發
)

const handleSave = async () => {
  if (!submit.value) return
  isSaving.value = true

  try {
    // 1. 將 UI 的修改同步回 changedTagsArray，這樣 innerSubmit 閉包裡的 changedTagsArray.value 才會是最新的
    changedTagsArray.value = editingTags.value

    // 2. 執行 innerSubmit (它會比較 changedTagsArray 和閉包裡的 defaultTags)
    await submit.value()
  } catch (error) {
    console.error('Failed to save tags:', error)
  } finally {
    isSaving.value = false
  }
}
</script>
