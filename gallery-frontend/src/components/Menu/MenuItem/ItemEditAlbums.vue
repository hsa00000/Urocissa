<template>
  <v-list-item
    prepend-icon="mdi-image-album"
    value="edit-albums"
    @click="modalStore.showEditAlbumsModal = true"
  >
    <v-list-item-title class="wrap">{{ 'Edit Albums' }}</v-list-item-title>
  </v-list-item>
</template>
<script setup lang="ts">
import { useModalStore } from '@/store/modalStore'

const modalStore = useModalStore('mainId')
</script>
