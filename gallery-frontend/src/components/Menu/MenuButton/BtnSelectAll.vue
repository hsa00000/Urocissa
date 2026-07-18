<template>
  <v-btn icon="mdi-checkbox-intermediate" aria-label="Select all" @click="selectAll"></v-btn>
</template>

<script lang="ts" setup>
import { IsolationId } from '@type/types'
import { useCollectionStore } from '@/store/collectionStore'

const props = defineProps<{
  isolationId: IsolationId
}>()

const collectionStore = useCollectionStore(props.isolationId)

const selectAll = () => {
  collectionStore.selectAll()
}
</script>
