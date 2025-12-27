<template>
  <v-list-item prepend-icon="mdi-archive-arrow-down" @click="setAsCover()">
    <v-list-item-title class="wrap">Set as Cover</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { useCollectionStore } from '@/store/collectionStore'
import { useDataStore } from '@/store/dataStore'
import { getIsolationIdByRoute } from '@utils/getter'
import axios from 'axios'
import { refreshAlbumMetadata } from '@utils/refreshAlbumMetadata'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const collectionStore = useCollectionStore(isolationId)
const dataStore = useDataStore(isolationId)

const setAsCover = async () => {
  if (collectionStore.editModeCollection.size !== 1) {
    console.warn('editModeCollection must contain exactly one item to set as cover.')
    return
  }

  const coverIndex = Array.from(collectionStore.editModeCollection)[0]
  if (coverIndex === undefined) {
    return
  }

  const data = dataStore.data.get(coverIndex)
  const coverHash = data?.type === 'image' || data?.type === 'video' ? data.id : undefined
  if (coverHash === undefined) {
    return
  }

  const albumId = route.params.hash

  if (typeof albumId !== 'string') {
    return
  }

  await axios.put(
    '/put/set_album_cover',
    {
      albumId: albumId,
      coverHash: coverHash
    },
    {
      headers: {
        'Content-Type': 'application/json'
      }
    }
  )

  await refreshAlbumMetadata(albumId)
  collectionStore.editModeOn = false
}
</script>
