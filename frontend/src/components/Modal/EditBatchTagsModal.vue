<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showBatchEditTagsModal"
    variant="flat"
    persistent
    id="batch-edit-tag-overlay"
  >
    <v-confirm-edit
      v-model="changedTags"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showBatchEditTagsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
          <template #title>Edit&nbsp;Tags</template>

          <template #text>
            <v-form
              ref="formRef"
              v-model="formIsValid"
              @submit.prevent="submit"
              validate-on="input"
            >
              <v-container>
                <v-combobox
                  v-model="proxyModel.value.add"
                  chips
                  multiple
                  label="Add Tags"
                  :items="availableTags"
                  :rules="[addTagsRule]"
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  autocomplete="off"
                />
              </v-container>

              <v-container>
                <v-combobox
                  v-model="proxyModel.value.remove"
                  chips
                  multiple
                  label="Remove Tags"
                  :items="availableTags"
                  :rules="[removeTagsRule]"
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  autocomplete="off"
                />
              </v-container>
            </v-form>
          </template>

          <v-divider />

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
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useTagStore } from '@/store/tagStore'
import { getIsolationIdByRoute } from '@utils/getter'
import type { VForm } from 'vuetify/components'
import { editTags } from '@/api/editTags'

interface ChangedTags {
  add: string[]
  remove: string[]
}

const formRef = ref<VForm | null>(null)
const formIsValid = ref(false)
const changedTags = ref<ChangedTags>({ add: [], remove: [] })

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore(isolationId)
const tagStore = useTagStore('mainId')

const availableTags = computed(() => tagStore.tags.map((t) => t.tag))

const addTagsRule = (arr: string[]) =>
  arr.every((t) => !changedTags.value.remove.includes(t)) ||
  'Some tags are already selected in Remove Tags'

const removeTagsRule = (arr: string[]) =>
  arr.every((t) => !changedTags.value.add.includes(t)) ||
  'Some tags are already selected in Add Tags'

const submit = ref<() => Promise<void> | undefined>()

onMounted(() => {
  submit.value = async () => {
    const hashes = Array.from(collectionStore.editModeCollection)
    await editTags(hashes, changedTags.value.add, changedTags.value.remove, isolationId)
    modalStore.showBatchEditTagsModal = false
  }
})

watch(
  () => [changedTags.value.add, changedTags.value.remove],
  async () => {
    await formRef.value?.validate()
  },
  { deep: true }
)
</script>
