<template>
  <v-btn icon="mdi-checkbox-blank-outline" @click="selectClear"></v-btn>
</template>

<script lang="ts" setup>
import { useCollectionStore } from '@/store/collectionStore'
import { IsolationId } from '@/script/common/types'

const props = defineProps<{
  isolationId: IsolationId
}>()

const collectionStore = useCollectionStore(props.isolationId)

const selectClear = () => {
  collectionStore.editModeCollection.clear()
}
</script>
