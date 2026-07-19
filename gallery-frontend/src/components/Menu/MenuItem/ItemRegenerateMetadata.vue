<template>
  <v-list-item prepend-icon="mdi-image-refresh-outline" @click="openReindexModal">
    <v-list-item-title class="wrap">Reindex</v-list-item-title>
  </v-list-item>
</template>

<script setup lang="ts">
import { useRoute } from 'vue-router'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useModalStore } from '@/store/modalStore'
import { useMessageStore } from '@/store/messageStore'
import { getIsolationIdByRoute } from '@utils/getter'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection, selectionCount } from '@/type/selection'

const props = defineProps<{
  indexList: SelectionInput
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const prefetchStore = usePrefetchStore(isolationId)
const modalStore = useModalStore('mainId')
const messageStore = useMessageStore('mainId')

const openReindexModal = () => {
  if (prefetchStore.timestamp === null) {
    messageStore.error('The current gallery snapshot is not ready yet')
    return
  }
  const selection = normalizeSelection(props.indexList)
  modalStore.openReindex({
    selection,
    timestamp: prefetchStore.timestamp,
    isolationId,
    targetCount: selectionCount(selection, prefetchStore.dataLength)
  })
}
</script>
