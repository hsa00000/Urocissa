<script setup lang="ts">
import BaseModal from '@/components/Modal/BaseModal.vue'
import type { SavedSearch } from '@/type/types'

withDefaults(
  defineProps<{
    search: SavedSearch | null
    loading?: boolean
  }>(),
  {
    loading: false
  }
)

const isOpen = defineModel<boolean>({ required: true })
const emit = defineEmits<{
  confirm: []
}>()
</script>

<template>
  <BaseModal v-model="isOpen" title="Delete Search" :loading="loading" :width="400">
    <v-alert
      icon="mdi-delete-outline"
      color="warning"
      variant="tonal"
      :title="search?.name ?? ''"
      text="This search will be removed from the drawer."
    />

    <template #actions>
      <v-spacer />
      <v-btn variant="text" :disabled="loading" @click="isOpen = false">Cancel</v-btn>
      <v-btn
        prepend-icon="mdi-delete-outline"
        color="error"
        variant="flat"
        :loading="loading"
        @click="emit('confirm')"
      >
        Delete
      </v-btn>
    </template>
  </BaseModal>
</template>
