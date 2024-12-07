<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showEditTagsModal"
    variant="flat"
    persistent
    id="edit-tag-overlay"
  >
    <v-card class="mx-auto w-100" max-width="400" variant="elevated" retain-focus>
      <v-form v-model="formIsValid" @submit.prevent="submit" validate-on="input">
        <v-card-title> Edit Tags </v-card-title>
        <v-container>
          <v-combobox
            v-model="changedTagsArray"
            chips
            multiple
            item-title="tag"
            item-value="tag"
            :items="filteredTagList"
            label="Tags"
            closable-chips
          ></v-combobox>
        </v-container>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            @click="modalStore.showEditTagsModal = false"
          >
            Cancel
          </v-btn>
          <v-btn
            color="teal-accent-4"
            variant="outlined"
            class="ma-2 button button-submit"
            :loading="!tagStore.fetched"
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
 * This modal is used for editing the tags of a single photo on the single photo view page.
 */
import { ref, computed, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useTagStore } from '@/store/tagStore'
import { editTagsInWorker } from '@/script/inWorker/editTagsInWorker'
import { getHashIndexDataFromRoute, getIsolationIdByRoute } from '@/script/common/functions'

const formIsValid = ref(false)
const changedTagsArray = ref<string[]>([])
const submit = ref<(() => void) | undefined>(undefined)

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const modalStore = useModalStore('mainId')
const tagStore = useTagStore(isolationId)

const tagList = computed(() => tagStore.tags)
const filteredTagList = computed(() =>
  tagList.value.filter((tag) => !specialTag(tag.tag)).map((tag) => tag.tag)
)

const specialTag = (tag: string): boolean => {
  return tag === '_archived' || tag === '_favorite'
}

onMounted(() => {
  const useSubmit = (): undefined | (() => void) => {
    const initializeResult = getHashIndexDataFromRoute(route)
    if (initializeResult === undefined) {
      console.error(
        "useSubmit Error: Failed to initialize result. 'getHashIndexDataFromRoute(route)' returned undefined."
      )
      return undefined
    }
    const { index, data } = initializeResult
    let defaultTags: string[]
    if (data.database) {
      defaultTags = data.database.tag
    } else if (data.album) {
      defaultTags = data.album.tag
    } else {
      console.error("useSubmit Error: 'data.database' is undefined.")
      return undefined
    }
    changedTagsArray.value = defaultTags

    const innerSubmit = () => {
      const hashArray: number[] = [index]
      const addTagsArrayComputed = changedTagsArray.value.filter(
        (tag) => !specialTag(tag) && !defaultTags.includes(tag)
      )
      const removeTagsArrayComputed = defaultTags.filter(
        (tag) => !specialTag(tag) && !changedTagsArray.value.includes(tag)
      )

      const isolationId = getIsolationIdByRoute(route)

      editTagsInWorker(hashArray, addTagsArrayComputed, removeTagsArrayComputed, isolationId)
    }
    return innerSubmit
  }
  submit.value = useSubmit()
})
</script>

<style scoped></style>
