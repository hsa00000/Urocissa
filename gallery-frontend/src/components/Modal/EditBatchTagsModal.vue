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

const formIsValid = ref(false)
const isSaving = ref(false)

const addedTags = ref<string[]>([])
const removedTags = ref<string[]>([])

const allTags = computed(() => tagStore.tags.map((t) => t.tag))
const selectedCount = computed(() => collectionStore.editModeCollection.size)

const hasChanges = computed(() => {
  return addedTags.value.length > 0 || removedTags.value.length > 0
})

const rules = {
  noEmpty: (v: string[]) => {
    const hasEmpty = v.some((tag) => !tag || tag.trim() === '')
    return !hasEmpty || 'Tags cannot be empty.'
  },

  /**
   * Cross-field Validation:
   * Prevents the same tag from appearing in both "Add" and "Remove" lists simultaneously
   * to avoid ambiguous state resolution on the backend.
   */
  noConflictAdded: (v: string[]) => {
    const conflicts = v.filter((tag) => removedTags.value.includes(tag))
    if (conflicts.length > 0) {
      return `Conflict: '${conflicts[0]}' is also in the remove list.`
    }
    return true
  },

  noConflictRemoved: (v: string[]) => {
    const conflicts = v.filter((tag) => addedTags.value.includes(tag))
    if (conflicts.length > 0) {
      return `Conflict: '${conflicts[0]}' is also in the add list.`
    }
    return true
  }
}

const initializeData = () => {
  addedTags.value = []
  removedTags.value = []
  formIsValid.value = true
}

watch(
  () => modalStore.showBatchEditTagsModal,
  (isOpen) => {
    if (isOpen) {
      initializeData()
    }
  }
)

const handleSave = async () => {
  if (!hasChanges.value || !formIsValid.value) return

  isSaving.value = true
  try {
    const selectedHashes = Array.from(collectionStore.editModeCollection)

    await editTags(selectedHashes, addedTags.value, removedTags.value, isolationId)

    messageStore.success('Batch update tags successful.')
    modalStore.showBatchEditTagsModal = false

    // Resets UI selection state after successful batch operation
    collectionStore.leaveEdit()
  } catch (e) {
    console.error(e)
    messageStore.error('Batch update tags failed.')
  } finally {
    isSaving.value = false
  }
}
</script>
