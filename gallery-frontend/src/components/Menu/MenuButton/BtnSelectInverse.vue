<template>
  <v-btn icon="mdi-checkbox-intermediate-variant" @click="selectInverse"></v-btn>
</template>

<script lang="ts" setup>
import { useCollectionStore } from '@/store/collectionStore'
import { IsolationId } from '@type/types'
const props = defineProps<{
  isolationId: IsolationId
}>()

const collectionStore = useCollectionStore(props.isolationId)

const selectInverse = () => {
  collectionStore.invertSelection()
}
</script>
