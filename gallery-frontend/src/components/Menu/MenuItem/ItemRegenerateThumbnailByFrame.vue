<template>
  <v-list-item prepend-icon="mdi-image-refresh-outline" @click="regenerateThumbnailByFrame">
    <v-list-item-title class="wrap">Capture Frame as Thumb</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import axios from 'axios'
import { getIsolationIdByRoute, getRouteResourceId } from '@utils/getter'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import { useMessageStore } from '@/store/messageStore'
import { useEditStore } from '@/store/editStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const currentFrameStore = useCurrentFrameStore(isolationId)
const messageStore = useMessageStore('mainId')
const editStore = useEditStore('mainId')

const regenerateThumbnailByFrame = async () => {
  const hash = getRouteResourceId(route)
  if (hash === undefined) return

  if (editStore.hasRegenerate(hash)) return

  editStore.addRegenerate(hash)
  try {
    await tryWithMessageStore(isolationId, async () => {
      const currentFrameBlob = await currentFrameStore.getCapture()
      if (currentFrameBlob) {
        const formData = new FormData()

        // Append the hash first
        formData.append('hash', hash)

        // Append the frame file
        formData.append('frame', currentFrameBlob)
        messageStore.info('Regenerating thumbnail...')

        const response = await axios.put('/put/regenerate-thumbnail-with-frame', formData, {
          headers: {
            'Content-Type': 'multipart/form-data'
          }
        })

        messageStore.success('Regenerating thumbnail successfully')
        console.log('Response:', response.data)
      }
    })
  } finally {
    editStore.removeRegenerate(hash)
  }
}
</script>
