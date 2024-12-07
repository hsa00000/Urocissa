<template>
  <v-dialog
    v-model="modalStore.showBatchEditTagsModal"
    variant="flat"
    persistent
    id="batch-edit-tag-overlay"
  >
    <v-card class="mx-auto w-100" max-width="400" variant="elevated">
      <v-card-title class="text-h5"> Edit Tags </v-card-title>
      <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
        <v-container>
          <v-combobox
            v-model="addTagsArray"
            chips
            multiple
            label="Add Tags"
            :rules="[addTagsRule]"
            :items="tagList.filter((tag) => !specialTag(tag))"
          ></v-combobox>
        </v-container>
        <v-container>
          <v-combobox
            v-model="removeTagsArray"
            chips
            multiple
            label="Remove Tags"
            :rules="[removeTagsRule]"
            :items="tagList.filter((tag) => !specialTag(tag))"
          ></v-combobox>
        </v-container>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            @click="modalStore.showBatchEditTagsModal = false"
          >
            Cancel
          </v-btn>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            :disabled="!formIsValid"
            type="submit"
          >
            Submit
          </v-btn>
        </v-card-actions>
      </v-form>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
/**
 * This modal is used for editing the tags of multiple photos on the home page.
 */
import { ref, computed } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useTagStore } from '@/store/tagStore'
import { editTagsInWorker } from '@/script/inWorker/editTagsInWorker'
import { getIsolationIdByRoute } from '@/script/common/functions'

const formIsValid = ref(false)
const addTagsArray = ref<string[]>([])
const removeTagsArray = ref<string[]>([])

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore(isolationId)
const tagStore = useTagStore(isolationId)

const tagList = computed(() => {
  return tagStore.tags.map((tag) => tag.tag)
})

const specialTag = (tag: string): boolean => {
  return tag == '_archived' || tag == '_favorite'
}

const addTagsRule = (inputArray: string[]) =>
  inputArray.every((tag) => !removeTagsArray.value.includes(tag)) ||
  'Some tags are already selected in Remove Tags'

const removeTagsRule = (inputArray: string[]) =>
  inputArray.every((tag) => !addTagsArray.value.includes(tag)) ||
  'Some tags are already selected in Add Tags'

const submit = () => {
  const hashArray = Array.from(collectionStore.editModeCollection)
  const isolationId = getIsolationIdByRoute(route)
  editTagsInWorker(hashArray, addTagsArray.value, removeTagsArray.value, isolationId)
  modalStore.showBatchEditTagsModal = false
}
</script>

<style scoped></style>
