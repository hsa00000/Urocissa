import { useDataStore } from '@/store/dataStore'
import { IsolationId, MessageColor, SlicedDataItem } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useMessageStore } from '@/store/messageStore'
import { useTagStore } from '@/store/tagStore'
import { createHandler } from 'typesafe-agent-events'
import { fromDataWorker } from '@/worker/workerApi'
import { useOffsetStore } from '@/store/offsetStore'
import { useRowStore } from '@/store/rowStore'
import { useLocationStore } from '@/store/locationStore'
import { useModalStore } from '@/store/modalStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { useRedirectionStore } from '@/store/redirectionStore'

const workerHandlerMap = new Map<Worker, (e: MessageEvent) => void>()

export function handleDataWorkerReturn(dataWorker: Worker, isolationId: IsolationId) {
  const messageStore = useMessageStore('mainId')
  const modalStore = useModalStore('mainId')
  const redirectionStore = useRedirectionStore('mainId')

  const dataStore = useDataStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const tagStore = useTagStore('mainId')
  const offsetStore = useOffsetStore(isolationId)
  const rowStore = useRowStore(isolationId)
  const locationStore = useLocationStore(isolationId)
  const optimisticUpateStore = useOptimisticStore(isolationId)

  const handler = createHandler<typeof fromDataWorker>({
    returnData: (payload) => {
      const slicedDataArray: SlicedDataItem[] = payload.slicedDataArray
      slicedDataArray.forEach(({ index, data }) => {
        dataStore.data.set(index, data)
        if (data.database) {
          dataStore.hashMapData.set(data.database.hash, index)
        } else if (data.album) {
          dataStore.hashMapData.set(data.album.id, index)
        }
      })
      dataStore.batchFetched.set(payload.batch, true)
      optimisticUpateStore.selfUpdate()
    },
    fetchRowReturn: (payload) => {
      const timestamp = payload.timestamp
      const rowWithOffset = payload.rowWithOffset
      const windowWidth = rowWithOffset.windowWidth
      if (windowWidth !== prefetchStore.windowWidth) {
        return
      }

      const offset = rowWithOffset.offset

      const row = rowWithOffset.row
      if (locationStore.anchor !== null && locationStore.anchor !== row.rowIndex) {
        return
      }
      const index = row.rowIndex
      if (timestamp === prefetchStore.timestamp && !offsetStore.offset.has(index)) {
        offsetStore.offset.set(index, offset)
        row.offset = offsetStore.accumulatedOffset(row.rowIndex)
        rowStore.rowData.forEach((row) => {
          if (row.rowIndex > index) {
            row.offset = row.offset + offset
          }
        })

        rowStore.rowData.set(row.rowIndex, row)

        prefetchStore.totalHeight = prefetchStore.totalHeight + offset
        offsetStore.accumulatedAll = offsetStore.accumulatedAll + offset
      }

      prefetchStore.updateFetchRowTrigger = !prefetchStore.updateFetchRowTrigger
      prefetchStore.updateVisibleRowTrigger = !prefetchStore.updateVisibleRowTrigger
    },
    editTagsReturn: (payload) => {
      if (payload.returnedTagsArray !== undefined) {
        tagStore.applyTags(payload.returnedTagsArray)
      } else {
        console.warn('Returned tags array is undefined')
      }
      modalStore.showEditTagsModal = false
    },
    notification: function (payload: { text: string; color: MessageColor }): void {
      messageStore.push(payload.text, payload.color)
    },
    unauthorized: async () => {
      await redirectionStore.redirectionToLogin()
    }
  })

  const messageHandler = (e: MessageEvent) => {
    handler(e.data as ReturnType<(typeof fromDataWorker)[keyof typeof fromDataWorker]>)
  }

  dataWorker.addEventListener('message', messageHandler)
  workerHandlerMap.set(dataWorker, messageHandler)
}

export function removeHandleDataWorkerReturn(dataWorker: Worker) {
  const messageHandler = workerHandlerMap.get(dataWorker)
  if (messageHandler) {
    dataWorker.removeEventListener('message', messageHandler)
    workerHandlerMap.delete(dataWorker)
  }
}
