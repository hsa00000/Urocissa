<template>
  <v-list-item
    prepend-icon="mdi-pencil-outline"
    value="edit-tags"
    @click="modalStore.showEditTagsModal = true"
  >
    <v-list-item-title class="wrap">{{ 'Edit Tags' }}</v-list-item-title>
  </v-list-item>
</template>
<script setup lang="ts">
import { useModalStore } from '@/store/modalStore'

const modalStore = useModalStore('mainId')
</script>
