<template>
  <v-list-item prepend-icon="mdi-image-refresh-outline" @click="reindex">
    <v-list-item-title class="wrap">Reindex</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { usePrefetchStore } from '@/store/prefetchStore'
import axios from 'axios'
import { getIsolationIdByRoute } from '@/script/common/functions'
import { useMessageStore } from '@/store/messageStore'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const prefetchStore = usePrefetchStore(isolationId)
const messageStore = useMessageStore('mainId')

const reindex = async () => {
  const indexArray = props.indexList
  const regenerateData = {
    indexArray: indexArray,
    timestamp: prefetchStore.timestamp
  }
  try {
    messageStore.showInfo('Reindexing...')
    await axios.post('/put/reindex', regenerateData, {
      headers: {
        'Content-Type': 'application/json'
      }
    })
    messageStore.showInfo('Regenerating metadata successfully')
  } catch (error) {
    messageStore.showWarn(`Regenerating metadata failed: ${String(error)}`)
  }
}
</script>
