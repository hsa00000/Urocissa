<template>
  <v-dialog
    v-if="submit !== undefined"
    v-model="modalStore.showBatchEditTagsModal"
    persistent
    id="batch-edit-tag-overlay"
    max-width="400"
  >
    <v-confirm-edit
      v-model="changedTags"
      :disabled="false"
      @save="submit"
      @cancel="modalStore.showBatchEditTagsModal = false"
    >
      <template #default="{ model: proxyModel, actions }">
        <v-card variant="elevated" retain-focus>
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
                  return-object
                  item-title="title"
                  item-value="value"
                  label="Add Tags"
                  :items="allItems"
                  :rules="[addTagsRule]"
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  autocomplete="off"
                >
                  <template #chip="{ props: chipProps, internalItem }">
                    <v-chip v-bind="chipProps">
                      <template v-if="isFlagItem(internalItem.raw)" #prepend>
                        <v-icon size="small" class="me-1">{{ internalItem.raw.icon }}</v-icon>
                      </template>
                      {{ internalItem.title }}
                    </v-chip>
                  </template>
                  <template #item="{ internalItem, props: itemProps }">
                    <v-list-item v-bind="itemProps">
                      <template #prepend="{ isActive }">
                        <v-list-item-action>
                          <v-checkbox-btn :model-value="isActive" />
                        </v-list-item-action>
                      </template>
                      <template #append>
                        <v-icon v-if="internalItem.raw.isFlag">{{ internalItem.raw.icon }}</v-icon>
                      </template>
                    </v-list-item>
                  </template>
                </v-combobox>
              </v-container>

              <v-container>
                <v-combobox
                  v-model="proxyModel.value.remove"
                  chips
                  multiple
                  return-object
                  item-title="title"
                  item-value="value"
                  label="Remove Tags"
                  :items="allItems"
                  :rules="[removeTagsRule]"
                  closable-chips
                  :menu-props="{ maxWidth: 0 }"
                  autocomplete="off"
                >
                  <template #chip="{ props: chipProps, internalItem }">
                    <v-chip v-bind="chipProps">
                      <template v-if="isFlagItem(internalItem.raw)" #prepend>
                        <v-icon size="small" class="me-1">{{ internalItem.raw.icon }}</v-icon>
                      </template>
                      {{ internalItem.title }}
                    </v-chip>
                  </template>
                  <template #item="{ internalItem, props: itemProps }">
                    <v-list-item v-bind="itemProps">
                      <template #prepend="{ isActive }">
                        <v-list-item-action>
                          <v-checkbox-btn :model-value="isActive" />
                        </v-list-item-action>
                      </template>
                      <template #append>
                        <v-icon v-if="internalItem.raw.isFlag">{{ internalItem.raw.icon }}</v-icon>
                      </template>
                    </v-list-item>
                  </template>
                </v-combobox>
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
/**
 * Batch edit modal for adding/removing tags across multiple selected items.
 *
 * Virtual flag items (isFavorite / isArchived) are mixed into both the "Add Tags"
 * and "Remove Tags" comboboxes via `return-object`. On save, real tags and flag
 * items are separated: real tags go through `editTags`, flag changes go through
 * `editFlags`. Placing a flag in "Add" sets it to true; placing it in "Remove"
 * sets it to false. Validation rules prevent the same item from appearing in both.
 */
import { ref, computed, watch, onMounted } from 'vue'
import { useRoute } from 'vue-router'
import { useModalStore } from '@/store/modalStore'
import { useCollectionStore } from '@/store/collectionStore'
import { useTagStore } from '@/store/tagStore'
import { getIsolationIdByRoute } from '@utils/getter'
import type { VForm } from 'vuetify/components'
import { editTags } from '@/api/editTags'
import { editFlags } from '@/api/editFlags'

// Combobox item shape used by both real tags and virtual flag items.
interface ComboboxItem {
  title: string
  value: string
  isFlag: boolean
  icon?: string
}

// With `return-object`, the combobox model contains ComboboxItem objects for items
// selected from the dropdown, and plain strings for user-typed free-text tags.
type ModelValue = string | ComboboxItem

// Virtual flag items — these map to boolean flags on the data object and are
// persisted via `editFlags`, not `editTags`.
const FAVORITE_ITEM: ComboboxItem = {
  title: 'Favorite',
  value: 'isFavorite',
  isFlag: true,
  icon: 'mdi-star'
}
const ARCHIVED_ITEM: ComboboxItem = {
  title: 'Archived',
  value: 'isArchived',
  isFlag: true,
  icon: 'mdi-archive-arrow-down'
}

// Type guard: returns true for virtual flag items.
function isFlagItem(v: ModelValue): v is ComboboxItem {
  return typeof v === 'object' && v.isFlag
}

// Extract the plain tag string from a model value.
function getTagString(v: ModelValue): string {
  return typeof v === 'string' ? v : v.value
}

interface ChangedTags {
  add: ModelValue[]
  remove: ModelValue[]
}

const formRef = ref<VForm | null>(null)
const formIsValid = ref(false)
const changedTags = ref<ChangedTags>({ add: [], remove: [] })

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const modalStore = useModalStore('mainId')
const collectionStore = useCollectionStore(isolationId)
const tagStore = useTagStore('mainId')

// Merge virtual flag items with real tags into a single dropdown list.
const allItems = computed<ComboboxItem[]>(() => {
  const tagItems = tagStore.tags.map((t) => ({ title: t.tag, value: t.tag, isFlag: false }))
  return [FAVORITE_ITEM, ARCHIVED_ITEM, ...tagItems]
})

// Validation: prevent the same item from appearing in both Add and Remove.
const addTagsRule = (arr: ModelValue[]) => {
  const removeKeys = new Set(changedTags.value.remove.map(getTagString))
  return (
    arr.every((t) => !removeKeys.has(getTagString(t))) ||
    'Some items are already selected in Remove Tags'
  )
}

const removeTagsRule = (arr: ModelValue[]) => {
  const addKeys = new Set(changedTags.value.add.map(getTagString))
  return (
    arr.every((t) => !addKeys.has(getTagString(t))) ||
    'Some items are already selected in Add Tags'
  )
}

const submit = ref<() => Promise<void> | undefined>()

onMounted(() => {
  submit.value = async () => {
    const selection = collectionStore.descriptor()
    const addValues = changedTags.value.add
    const removeValues = changedTags.value.remove

    // Split model: separate real tags from virtual flag items.
    const addTagsArray = addValues.filter((v) => !isFlagItem(v)).map(getTagString)
    const removeTagsArray = removeValues.filter((v) => !isFlagItem(v)).map(getTagString)

    modalStore.showBatchEditTagsModal = false

    // Persist real tag changes via editTags (with optimistic update).
    if (addTagsArray.length > 0 || removeTagsArray.length > 0) {
      await editTags(selection, addTagsArray, removeTagsArray, isolationId)
    }

    // Persist flag changes via editFlags.
    // Flags in "Add" → set true; flags in "Remove" → set false.
    const flagChanges: { isFavorite?: boolean; isArchived?: boolean } = {}
    if (addValues.some((v) => isFlagItem(v) && v.value === 'isFavorite'))
      flagChanges.isFavorite = true
    if (addValues.some((v) => isFlagItem(v) && v.value === 'isArchived'))
      flagChanges.isArchived = true
    if (removeValues.some((v) => isFlagItem(v) && v.value === 'isFavorite'))
      flagChanges.isFavorite = false
    if (removeValues.some((v) => isFlagItem(v) && v.value === 'isArchived'))
      flagChanges.isArchived = false

    if (Object.keys(flagChanges).length > 0) {
      await editFlags(selection, flagChanges, isolationId)
    }
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
