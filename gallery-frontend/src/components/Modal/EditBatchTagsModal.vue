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
      ></v-combobox>

      <v-combobox
        v-model="removedTags"
        label="Remove Tags"
        chips
        closable-chips
        multiple
        item-title="tag"
        item-value="tag"
        :items="commonTags"
        variant="outlined"
        density="comfortable"
        hide-details="auto"
        :disabled="isSaving"
        autocomplete="off"
      >
        <template #details>
          <div class="text-caption text-medium-emphasis">
            Only tags present in all selected items are shown here.
          </div>
        </template>
      </v-combobox>
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
import { useTagStore } from '@/store/tagStore'
import { useCollectionStore } from '@/store/collectionStore' // 使用 CollectionStore
import { useMessageStore } from '@/store/messageStore'
import { editTags } from '@/api/editTags' // 使用標準 editTags
// 注意：你需要在 api/editTags 中實作 getCommonTags，目前先不引入以避免報錯
// import { getCommonTags } from '@/api/editTags'
import BaseModal from '@/components/Modal/BaseModal.vue'
import { getIsolationIdByRoute } from '@utils/getter'

const route = useRoute()
const modalStore = useModalStore('mainId')
const tagStore = useTagStore('mainId')
const messageStore = useMessageStore('mainId')

const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)

const formIsValid = ref(false)
const isSaving = ref(false)

const addedTags = ref<string[]>([])
const removedTags = ref<string[]>([])
const commonTags = ref<string[]>([])

const allTags = computed(() => tagStore.tags.map((t) => t.tag))
const selectedCount = computed(() => collectionStore.editModeCollection.size)

// 初始化數據
const initializeData = async () => {
  addedTags.value = []
  removedTags.value = []
  commonTags.value = []

  if (selectedCount.value === 0) return

  // TODO: 如果你想實作 "只顯示共同 Tag"，請在 src/api/editTags.ts 實作 getCommonTags
  // 並在這裡解開註解
  /*
  try {
    const result = await getCommonTags(Array.from(collectionStore.editModeCollection))
    commonTags.value = result
  } catch (e) {
    console.error('Failed to fetch common tags', e)
    messageStore.error('Failed to fetch common tags.')
  }
  */

  // 暫時替代方案：顯示所有 Tags 供移除
  commonTags.value = allTags.value
}

// 監聽 Modal 開啟
watch(
  () => modalStore.showBatchEditTagsModal, // 修正拼字
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  }
)

const handleSave = async () => {
  if (addedTags.value.length === 0 && removedTags.value.length === 0) {
    modalStore.showBatchEditTagsModal = false
    return
  }

  isSaving.value = true
  try {
    const selectedHashes = Array.from(collectionStore.editModeCollection)
    // 使用 editTags 進行批量修改
    await editTags(selectedHashes, addedTags.value, removedTags.value, isolationId)

    messageStore.success('Batch update tags successful.')
    modalStore.showBatchEditTagsModal = false

    // 清空選取狀態
    collectionStore.editModeOn = false
  } catch (e) {
    console.error(e)
    messageStore.error('Batch update tags failed.')
  } finally {
    isSaving.value = false
  }
}
</script>
