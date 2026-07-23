<script setup lang="ts">
import Sortable from 'sortablejs'
import { computed, nextTick, onBeforeUnmount, onMounted, shallowRef, useId, watch } from 'vue'
import { useRoute } from 'vue-router'
import { useDisplay } from 'vuetify'
import { useMessageStore } from '@/store/messageStore'
import { useSavedSearchStore } from '@/store/savedSearchStore'
import SavedSearchDeleteDialog from './SavedSearchDeleteDialog.vue'
import SavedSearchNameDialog from './SavedSearchNameDialog.vue'
import { createSavedSearchLocation, isSavedSearchActive } from './savedSearchRoute'
import type { SavedSearch } from '@/type/types'

const props = withDefaults(
  defineProps<{
    disabled?: boolean
  }>(),
  {
    disabled: false
  }
)

const route = useRoute()
const { mdAndUp } = useDisplay()
const messageStore = useMessageStore('mainId')
const savedSearchStore = useSavedSearchStore()
const savedSearchListId = useId()
const renamingSearch = shallowRef<SavedSearch | null>(null)
const deletingSearch = shallowRef<SavedSearch | null>(null)
const showRenameDialog = shallowRef(false)
const showDeleteDialog = shallowRef(false)
let sortable: Sortable | null = null

const interactionDisabled = computed(
  () => props.disabled || savedSearchStore.loading || savedSearchStore.mutating
)
const renameExistingNames = computed(() => {
  const currentId = renamingSearch.value?.id
  return savedSearchStore.searches
    .filter((search) => search.id !== currentId)
    .map((search) => search.name)
})

function openRenameDialog(search: SavedSearch): void {
  renamingSearch.value = search
  showRenameDialog.value = true
}

async function renameSearch(name: string): Promise<void> {
  const search = renamingSearch.value
  if (search === null) return

  const succeeded = await savedSearchStore.rename(search.id, name)
  if (!succeeded) return

  showRenameDialog.value = false
  renamingSearch.value = null
  messageStore.success('Search renamed.')
}

function openDeleteDialog(search: SavedSearch): void {
  deletingSearch.value = search
  showDeleteDialog.value = true
}

async function deleteSearch(): Promise<void> {
  const search = deletingSearch.value
  if (search === null) return

  const succeeded = await savedSearchStore.remove(search.id)
  if (!succeeded) return

  showDeleteDialog.value = false
  deletingSearch.value = null
  messageStore.success('Search deleted.')
}

async function applyDraggedOrder(ids: string[]): Promise<void> {
  const sortableInstance = sortable
  if (sortableInstance === null) return

  const succeeded = await savedSearchStore.reorder(ids)
  if (!succeeded && sortable === sortableInstance) {
    sortableInstance.sort(savedSearchStore.searches.map((search) => search.id))
  }
}

function createSortable(): void {
  const listElement = document.getElementById(savedSearchListId)
  if (!(listElement instanceof HTMLElement) || sortable !== null) return

  sortable = Sortable.create(listElement, {
    animation: 180,
    dataIdAttr: 'data-saved-search-id',
    delay: mdAndUp.value ? 0 : 350,
    delayOnTouchOnly: false,
    disabled: interactionDisabled.value,
    draggable: '.saved-search-item',
    dragClass: 'elevation-4',
    fallbackOnBody: true,
    fallbackTolerance: 4,
    filter: '.saved-search-actions',
    forceFallback: true,
    ghostClass: 'opacity-50',
    preventOnFilter: false,
    touchStartThreshold: 5,
    onEnd: () => {
      const sortableInstance = sortable
      if (sortableInstance !== null) {
        void applyDraggedOrder(sortableInstance.toArray())
      }
    }
  })
}

watch(interactionDisabled, (disabled) => {
  sortable?.option('disabled', disabled)
})

watch(mdAndUp, (desktop) => {
  sortable?.option('delay', desktop ? 0 : 350)
})

onMounted(async () => {
  await nextTick()
  createSortable()
})

onBeforeUnmount(() => {
  sortable?.destroy()
  sortable = null
})
</script>

<template>
  <v-list
    :id="savedSearchListId"
    nav
    :disabled="props.disabled"
    data-testid="saved-search-list"
    aria-label="Saved searches"
  >
    <v-list-item
      v-for="search in savedSearchStore.searches"
      :key="search.id"
      :data-saved-search-id="search.id"
      :to="createSavedSearchLocation(search)"
      :active="isSavedSearchActive(search, route)"
      :title="search.name"
      :aria-label="`${search.name}. Drag to reorder.`"
      class="saved-search-item"
      prepend-icon="mdi-bookmark-outline"
      slim
    >
      <template #append>
        <v-menu>
          <template #activator="{ props: menuProps }">
            <v-btn
              v-bind="menuProps"
              class="saved-search-actions"
              icon="mdi-dots-vertical"
              size="x-small"
              variant="text"
              :aria-label="`Manage ${search.name}`"
              :disabled="interactionDisabled"
              @click.prevent.stop
            />
          </template>

          <v-list density="compact">
            <v-list-item
              prepend-icon="mdi-pencil-outline"
              title="Rename"
              @click="openRenameDialog(search)"
            />
            <v-list-item
              prepend-icon="mdi-delete-outline"
              title="Delete"
              base-color="error"
              @click="openDeleteDialog(search)"
            />
          </v-list>
        </v-menu>
      </template>
    </v-list-item>
  </v-list>

  <SavedSearchNameDialog
    v-if="renamingSearch !== null"
    v-model="showRenameDialog"
    title="Rename Search"
    :initial-name="renamingSearch.name"
    :existing-names="renameExistingNames"
    :loading="savedSearchStore.mutating"
    @submit="renameSearch"
  />

  <SavedSearchDeleteDialog
    v-model="showDeleteDialog"
    :search="deletingSearch"
    :loading="savedSearchStore.mutating"
    @confirm="deleteSearch"
  />
</template>

<style scoped>
.saved-search-item {
  cursor: grab;
}

.saved-search-item:active {
  cursor: grabbing;
}
</style>
