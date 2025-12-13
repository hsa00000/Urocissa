<template>
  <BaseModal
    v-if="submit !== undefined"
    v-model="modalStore.showEditAlbumsModal"
    title="Edit Albums"
    width="400"
    :loading="isSaving"
  >
    <v-form v-model="formIsValid" @submit.prevent="handleSave">
      <v-combobox
        v-model="editingAlbums"
        chips
        multiple
        item-title="title"
        item-value="id"
        :items="albumList"
        label="Albums"
        closable-chips
        variant="outlined"
        autocomplete="off"
        hide-details
        :disabled="isSaving"
        :return-object="false"
      />
    </v-form>

    <template #actions>
      <v-spacer />
      <v-btn variant="text" :disabled="isSaving" @click="modalStore.showEditAlbumsModal = false">
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
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'
import { editAlbums } from '@/api/editAlbums'
import BaseModal from '@/components/Modal/BaseModal.vue'

const modalStore = useModalStore('mainId')
const albumStore = useAlbumStore('mainId')
const route = useRoute()

const formIsValid = ref(false)
const isSaving = ref(false)

// changedAlbumsArray: 這裡存放的是"當前最新的"相簿狀態 (在 save 時會被更新)
const changedAlbumsArray = ref<string[]>([])
// editingAlbums: 這裡存放 UI 編輯中的狀態
const editingAlbums = ref<string[]>([])

const submit = ref<(() => Promise<void>) | undefined>(undefined)

// 將 Map 轉為 Array，並映射成 { title, id } 格式
const albumList = computed(() =>
  Array.from(albumStore.albums.values()).map((a) => ({
    title: a.displayName || a.albumId,
    id: a.albumId
  }))
)

const initializeData = () => {
  const initializeResult = getHashIndexDataFromRoute(route)

  if (initializeResult === undefined) {
    console.warn('EditAlbumsModal: Failed to initialize result from route.')
    submit.value = undefined
    return
  }

  const { index, data } = initializeResult

  if (data.data.type === 'album') {
    console.warn('EditAlbumsModal: Cannot edit albums for an album entity.')
    submit.value = undefined
    return
  }

  const defaultAlbums: string[] = data.data.albums || []

  // 初始化 changedAlbumsArray (視為當前 DB 狀態)
  changedAlbumsArray.value = [...defaultAlbums]
  // 初始化 UI 顯示的 editingAlbums (複製一份)
  editingAlbums.value = [...defaultAlbums]

  const innerSubmit = async () => {
    const hashArray: number[] = [index]

    // 計算新增的相簿：存在於 changedAlbumsArray 但不在 defaultAlbums 中
    const addAlbumsArray = changedAlbumsArray.value.filter((id) => !defaultAlbums.includes(id))

    // 計算移除的相簿：存在於 defaultAlbums 但不在 changedAlbumsArray 中
    const removeAlbumsArray = defaultAlbums.filter((id) => !changedAlbumsArray.value.includes(id))

    const isolationId = getIsolationIdByRoute(route)
    modalStore.showEditAlbumsModal = false
    await editAlbums(hashArray, addAlbumsArray, removeAlbumsArray, isolationId)
  }

  submit.value = innerSubmit
}

watch(
  () => modalStore.showEditAlbumsModal,
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  },
  { immediate: true }
)

const handleSave = async () => {
  if (!submit.value) return
  isSaving.value = true

  try {
    // 1. 將 UI 的修改同步回 changedAlbumsArray
    changedAlbumsArray.value = editingAlbums.value
    // 2. 執行 innerSubmit
    await submit.value()
  } catch (error) {
    console.error('Failed to save albums:', error)
  } finally {
    isSaving.value = false
  }
}
</script>
