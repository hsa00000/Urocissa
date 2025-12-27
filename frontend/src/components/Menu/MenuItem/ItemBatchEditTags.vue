<template>
  <v-list-item prepend-icon="mdi-tag" @click="showBatchEditTagsModal">
    <v-list-item-title class="wrap">Batch Edit Tags</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useModalStore } from '@/store/modalStore'

const modalStore = useModalStore('mainId')

const showBatchEditTagsModal = () => {
  modalStore.showBatchEditTagsModal = true
}
</script>
