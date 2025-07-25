<template>
  <v-list-item prepend-icon="mdi-download" @click="downloadAllFiles">
    <v-list-item-title class="wrap">Download</v-list-item-title>
  </v-list-item>
</template>
<script lang="ts" setup>
import { useRoute } from 'vue-router'
import { useDataStore } from '@/store/dataStore'
import axios from 'axios'
import { saveAs } from 'file-saver'
import { fetchDataInWorker } from '@/api/fetchData'
import { getIsolationIdByRoute, getSrcOriginal } from '@utils/getter'
import { AbstractData } from '@type/types'
import { useTokenStore } from '@/store/tokenStore'
import { tryWithMessageStore } from '@/script/utils/try_catch'

const props = defineProps<{
  indexList: number[]
}>()

const route = useRoute()
const isolationId = getIsolationIdByRoute(route)
const dataStore = useDataStore(isolationId)
const tokenStore = useTokenStore(isolationId)

const waitForMetadata = (index: number, timeout = 5000, interval = 100): Promise<AbstractData> => {
  console.log(`data with index ${index} not fetch; waiting...`)

  return new Promise((resolve, reject) => {
    const startTime = Date.now()

    const checkMetadata = () => {
      const metadata = dataStore.data.get(index)

      if (metadata) {
        console.log(`index ${index} waiting done`)
        resolve(metadata)
      } else if (Date.now() - startTime > timeout) {
        console.error(`index ${index} waiting timeout`)
        reject(new Error(`Timeout waiting for metadata at index ${index}`))
      } else {
        setTimeout(checkMetadata, interval)
      }
    }
    checkMetadata()
  })
}

const downloadAllFiles = async () => {
  const indexArray = props.indexList
  const concurrencyLimit = 8
  const delay = 1000
  const delayFunction = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms))
  const isolationId = getIsolationIdByRoute(route)

  await tryWithMessageStore(isolationId, async () => {
    for (let i = 0; i < indexArray.length; i += concurrencyLimit) {
      const batchIndex = indexArray.slice(i, i + concurrencyLimit)
      const downloadPromises = batchIndex.map(async (index) => {
        let metadata = dataStore.data.get(index)
        if (!metadata) {
          // Initiate data fetch
          await fetchDataInWorker('single', index, isolationId)

          // Wait for metadata to be available
          metadata = await tryWithMessageStore(isolationId, async () => {
            return await waitForMetadata(index)
          })
          
          if (!metadata) {
            return // Skip this index if metadata isn't available
          }
        }

        if (metadata.database) {
          const hash = metadata.database.hash

          const url = getSrcOriginal(hash, true, metadata.database.ext)
          await tokenStore.tryRefreshAndStoreTokenToDb(hash)
          const hashToken = tokenStore.hashTokenMap.get(hash)
          if (hashToken === undefined) {
            console.error(`hashToken is undefined for hash: ${hash}`)
            return
          }

          const downloadResult = await tryWithMessageStore(isolationId, async () => {
            const response = await axios.get<Blob>(url, {
              responseType: 'blob',
              headers: {
                Authorization: `Bearer ${hashToken}`
              }
            })

            if (metadata.database) {
              const fileName = `${hash}.${metadata.database.ext}`
              saveAs(response.data, fileName)
            }
            return true
          })
          
          if (downloadResult === undefined) {
            console.error(`Failed to download file for index ${index}`)
          }
        }
      })

      await Promise.all(downloadPromises)
      await delayFunction(delay)
    }
    console.log('All files downloaded successfully')
  })
}
</script>
