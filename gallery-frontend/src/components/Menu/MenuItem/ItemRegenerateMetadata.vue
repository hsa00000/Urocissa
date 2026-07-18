<template>
  <v-list-item prepend-icon="mdi-image-refresh-outline" @click="reindex">
    <v-list-item-title class="wrap">Reindex</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { usePrefetchStore } from '@/store/prefetchStore'
import axios from 'axios'
import { getIsolationIdByRoute } from '@utils/getter'
import { useMessageStore } from '@/store/messageStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'

const props = defineProps<{
  indexList: SelectionInput
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const prefetchStore = usePrefetchStore(isolationId)
const messageStore = useMessageStore('mainId')

const reindex = async () => {
  const regenerateData = {
    selection: normalizeSelection(props.indexList),
    timestamp: prefetchStore.timestamp
  }
  
  await tryWithMessageStore('mainId', async () => {
    messageStore.info('Reindexing...')
    await axios.post('/put/reindex', regenerateData, {
      headers: {
        'Content-Type': 'application/json'
      }
    })
    messageStore.success('Regenerating metadata successfully')
  })
}
</script>
