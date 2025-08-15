<template>
  <v-toolbar flat height="2" class="no-select" :style="{ backgroundColor: 'rgb(var(--v-theme-surface))' }" />
  <InfoBar v-if="!collectionStore.editModeOn" />
  <EditBar v-else />
</template>

<script setup lang="ts">
import { useCollectionStore } from '@/store/collectionStore'
import EditBar from '@/components/NavBar/EditBar.vue'
import InfoBar from '@/components/NavBar/InfoBar.vue'

const collectionStore = useCollectionStore('mainId')
</script>
