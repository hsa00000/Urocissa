<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditTagsModal"
    variant="flat"
    persistent
    id="edit-tag-overlay"
  >
    <v-confirm-edit
      v-model="changedTagsArray"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showEditTagsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
          <template #title> Edit Tags </template>
          <template #text>
            <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
              <v-combobox
                v-model="proxyModel.value"
                chips
                multiple
                item-title="tag"
                item-value="tag"
                :items="filteredTagList"
                label="Tags"
                closable-chips
                variant="outlined"
                autocomplete="off"
              />
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
/**
 * This modal is used for editing the tags of a single photo on the single photo view page.
 */
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useTagStore } from '@/store/tagStore'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@utils/getter'
import { editTags } from '@/api/editTags'

const formIsValid = ref(false)
const changedTagsArray = ref<string[]>([])
const submit = ref<(() => Promise<void>) | undefined>(undefined)

const route = useRoute()
const modalStore = useModalStore('mainId')
const tagStore = useTagStore('mainId')

const tagList = computed(() => tagStore.tags)
const filteredTagList = computed(() => tagList.value.map((tag) => tag.tag))

onMounted(() => {
  const useSubmit = (): undefined | (() => Promise<void>) => {
    const initializeResult = getHashIndexDataFromRoute(route)
    if (initializeResult === undefined) {
      console.error(
        "useSubmit Error: Failed to initialize result. 'getHashIndexDataFromRoute(route)' returned undefined."
      )
      return undefined
    }
    const { index, data } = initializeResult
    let defaultTags: string[]
    if (data.type === 'image' || data.type === 'video') {
      defaultTags = data.tags
    } else if (data.type === 'album') {
      defaultTags = data.tags
    } else {
      console.error("useSubmit Error: 'data' type is not recognized.")
      return undefined
    }
    changedTagsArray.value = [...defaultTags]

    const innerSubmit = async () => {
      const hashArray: number[] = [index]
      const addTagsArrayComputed = changedTagsArray.value.filter(
        (tag) => !defaultTags.includes(tag)
      )
      const removeTagsArrayComputed = defaultTags.filter(
        (tag) => !changedTagsArray.value.includes(tag)
      )

      const isolationId = getIsolationIdByRoute(route)

      await editTags(hashArray, addTagsArrayComputed, removeTagsArrayComputed, isolationId)
      modalStore.showEditTagsModal = false
    }
    return innerSubmit
  }
  submit.value = useSubmit()
})
</script>
