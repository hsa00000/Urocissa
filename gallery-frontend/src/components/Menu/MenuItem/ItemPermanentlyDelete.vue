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
import type { CollectionIsolationId } from '@/type/types'
import {
  clearCachedResource,
  selectedCachedResourceIds
} from '@/script/utils/routeResourceCache'
import { useCollectionReloadStore } from '@/store/collectionReloadStore'
const route = useRoute()
const router = useRouter()
const isolationId = getIsolationIdByRoute(route)
const prefetchStore = usePrefetchStore(isolationId)
const messageStore = useMessageStore('mainId')
const collectionReloadStore = useCollectionReloadStore('mainId')
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
    const affectedCollections = new Set<CollectionIsolationId>()
    for (const resourceId of selectedIds) {
      for (const affected of clearCachedResource(resourceId)) {
        affectedCollections.add(affected)
      }
    }
    // A selection can include unloaded rows, so the source snapshot must be
    // refreshed even when no selected ID happened to be hydrated locally.
    if (isolationId === 'mainId' || isolationId === 'subId') {
      affectedCollections.add(isolationId)
    }
    if (affectedCollections.has('mainId')) {
      collectionReloadStore.requestMainCollectionReload()
    }
    if (affectedCollections.has('subId')) {
      collectionReloadStore.requestSubCollectionReload()
    }
    messageStore.success('Successfully deleted data.')
    if (routeResourceId !== undefined && selectedIds.includes(routeResourceId)) router.back()
  })
}
</script>
