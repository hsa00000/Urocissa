<template>
  <v-btn icon="mdi-close" @click="leaveEdit"></v-btn>
</template>

<script lang="ts" setup>
import { useCollectionStore } from '@/store/collectionStore'
import { useRoute } from 'vue-router'
import { getIsolationIdByRoute } from '@/script/common/functions'
const route = useRoute()
const isolationId = getIsolationIdByRoute(route)

const collectionStore = useCollectionStore(isolationId)

const leaveEdit = () => {
  collectionStore.editModeCollection.clear()
  collectionStore.editModeOn = false
}
</script>
