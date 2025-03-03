<template>
  <v-btn icon="mdi-checkbox-intermediate" @click="selectAll"></v-btn>
</template>

<script lang="ts" setup>
import { IsolationId } from '@/script/common/types'
import { useCollectionStore } from '@/store/collectionStore'
import { usePrefetchStore } from '@/store/prefetchStore'

const props = defineProps<{
  isolationId: IsolationId
}>()

const collectionStore = useCollectionStore(props.isolationId)
const prefetchStore = usePrefetchStore(props.isolationId)

const selectAll = () => {
  for (let i = 0; i < prefetchStore.dataLength; i++) {
    collectionStore.editModeCollection.add(i)
  }
}
</script>
