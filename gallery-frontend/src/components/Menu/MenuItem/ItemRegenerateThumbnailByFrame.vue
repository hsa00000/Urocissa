<template>
  <v-list-item prepend-icon="mdi-image-refresh-outline" @click="regenerateThumbnailByFrame">
    <v-list-item-title class="wrap">Capture Frame as Thumb</v-list-item-title>
  </v-list-item>
</template>

<script lang="ts" setup>
import { useRoute } from 'vue-router'
import axios from 'axios'
import { getCookiesJwt, getIsolationIdByRoute } from '@/script/common/functions'
import { useCurrentFrameStore } from '@/store/currentFrameStore'
import { getSrc } from '@/../config'
import { useMessageStore } from '@/store/messageStore'

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const currentFrameStore = useCurrentFrameStore(isolationId)
const messageStore = useMessageStore('mainId')

const regenerateThumbnailByFrame = async () => {
  try {
    const hash = route.params.hash
    const currentFrameBlob = await currentFrameStore.getCapture()
    if (typeof hash === 'string' && currentFrameBlob) {
      const formData = new FormData()

      // Append the hash first
      formData.append('hash', hash)

      // Append the file
      formData.append('file', currentFrameBlob)
      messageStore.showInfo('Regenerating thumbnail...')

      const response = await axios.put('/put/regenerate-thumbnail-with-frame', formData, {
        headers: {
          'Content-Type': 'multipart/form-data'
        }
      })

      const blobHeader = await fetch(getSrc(hash, false, 'jpg', getCookiesJwt(), undefined), {
        method: 'GET',
        cache: 'reload'
      })
      await blobHeader.blob()
      messageStore.showInfo('Regenerating thumbnail successfually')
      console.log('Response:', response.data)
    }
  } catch (err) {
    messageStore.showWarn(`Regenerating thumbnail failed ${String(err)}`)
  }
}
</script>
