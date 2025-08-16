<template>
  <InfoBar v-if="!collectionStore.editModeOn" />
  <EditBar v-else />
</template>

<script setup lang="ts">
import { useCollectionStore } from '@/store/collectionStore'
import EditBar from '@/components/NavBar/EditBar.vue'
import InfoBar from '@/components/NavBar/InfoBar.vue'

const collectionStore = useCollectionStore('mainId')
</script>
