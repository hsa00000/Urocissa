<template>
  <v-list-item prepend-icon="mdi-trash-can-outline" @click="deleteData">
    <v-list-item-title class="wrap">Permanently Delete</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute, useRouter } from 'vue-router'
import { getIsolationIdByRoute, getRouteResourceId } from '@utils/getter'
import { usePrefetchStore } from '@/store/prefetchStore'
import axios from 'axios'
import { useMessageStore } from '@/store/messageStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'
import type { SelectionInput } from '@/type/selection'
import { normalizeSelection } from '@/type/selection'
import {
  clearCachedResource,
  selectedCachedResourceIds
} from '@/script/utils/routeResourceCache'
const route = useRoute()
const router = useRouter()
const isolationId = getIsolationIdByRoute(route)
const prefetchStore = usePrefetchStore(isolationId)
const messageStore = useMessageStore('mainId')
const props = defineProps<{
  indexList: SelectionInput
}>()

const deleteData = async () => {
  const timestamp = prefetchStore.timestamp
  if (timestamp === null) return
  const selection = normalizeSelection(props.indexList)
  const selectedIds = selectedCachedResourceIds(isolationId, selection)

  await tryWithMessageStore('mainId', async () => {
    await axios.delete('/delete/delete-data', {
      data: { selection, timestamp }
    })
    const routeResourceId = getRouteResourceId(route)
    for (const resourceId of selectedIds) clearCachedResource(resourceId)
    messageStore.success('Successfully deleted data.')
    if (routeResourceId !== undefined && selectedIds.includes(routeResourceId)) router.back()
  })
}
</script>
