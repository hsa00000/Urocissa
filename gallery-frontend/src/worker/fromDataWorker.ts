import { useDataStore } from '@/store/dataStore'
import { IsolationId, SlicedData } from '@type/types'
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
import { useScrollTopStore } from '@/store/scrollTopStore'
import { useTokenStore } from '@/store/tokenStore'
import { useConstStore } from '@/store/constStore'

const workerHandlerMap = new Map<Worker, (e: MessageEvent) => void>()

/**
 * Establishes the event listeners for the DataWorker based on a specific isolation context.
 * Handles data synchronization, virtual scroll layout recalculations, and token management.
 *
 * @param dataWorker - The worker instance to attach listeners to.
 * @param isolationId - The unique context ID (e.g., tab or session) for store isolation.
 */
export function handleDataWorkerReturn(dataWorker: Worker, isolationId: IsolationId) {
  const messageStore = useMessageStore('mainId')
  const modalStore = useModalStore('mainId')
  const redirectionStore = useRedirectionStore('mainId')
  const tagStore = useTagStore('mainId')
  const constStore = useConstStore('mainId')
  const tokenStore = useTokenStore(isolationId)
  const dataStore = useDataStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const offsetStore = useOffsetStore(isolationId)
  const rowStore = useRowStore(isolationId)
  const locationStore = useLocationStore(isolationId)
  const scrollTopStore = useScrollTopStore(isolationId)
  const optimisticUpdateStore = useOptimisticStore(isolationId)

  const handler = createHandler<typeof fromDataWorker>({
    returnData: (payload) => {
      const slicedDataArray: SlicedData[] = payload.slicedDataArray
      slicedDataArray.forEach(({ index, data, hashToken }) => {
        dataStore.data.set(index, data)
        dataStore.hashMapData.set(data.id, index)

        // Business Logic: Albums rely on 'cover' for token validation,
        // whereas distinct media types (Images/Videos) use their unique ID.
        if (data.type === 'album') {
          if (data.cover !== null) {
            tokenStore.hashTokenMap.set(data.cover, hashToken)
          }
        } else {
          tokenStore.hashTokenMap.set(data.id, hashToken)
        }
      })
      dataStore.batchFetched.set(payload.batch, true)
      optimisticUpdateStore.selfUpdate()
    },

    fetchRowReturn: (payload) => {
      const { timestamp, rowWithOffset, subRowHeightScale, limitRatio } = payload
      const windowWidth = rowWithOffset.windowWidth

      // Discard calculation if viewport changed during worker processing to avoid layout trashing.
      if (windowWidth !== prefetchStore.windowWidth) {
        return
      }

      const offset = rowWithOffset.offset
      const row = rowWithOffset.row

      // Prevent updates if the view is locked (anchored) to a specific row to maintain scroll stability.
      if (locationStore.anchor !== null && locationStore.anchor !== row.rowIndex) {
        return
      }

      const index = row.rowIndex
      const timestampMatched = timestamp === prefetchStore.timestamp
      const offsetNotComputed = !offsetStore.offset.has(index)
      const subRowHeightScaleMatched = subRowHeightScale === constStore.subRowHeightScale
      const limitRatioMatched = limitRatio === constStore.limitRatio

      //
      // Why: If the computed height (offset) is valid and new, we must propagate this delta
      // to all subsequent rows to ensure the virtual scroll total height remains accurate.
      if (
        timestampMatched &&
        offsetNotComputed &&
        subRowHeightScaleMatched &&
        limitRatioMatched
      ) {
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

      // Second step of two-step locate jump: refine scroll to exact subrow position.
      const pendingTarget = locationStore.pendingLocateTarget
      if (pendingTarget !== null && pendingTarget >= row.start && pendingTarget <= row.end) {
        const elementIndex = pendingTarget - row.start
        const displayElement = row.displayElements[elementIndex]
        if (displayElement !== undefined) {
          scrollTopStore.scrollTop =
            row.topPixelAccumulated + row.offset + displayElement.displayTopPixelAccumulated
        }
        locationStore.pendingLocateTarget = null
        locationStore.highlightedIndex = pendingTarget
      }

      prefetchStore.updateFetchRowTrigger = !prefetchStore.updateFetchRowTrigger
      prefetchStore.updateVisibleRowTrigger = !prefetchStore.updateVisibleRowTrigger
      rowStore.firstRowFetched = true
    },

    editTagsReturn: (payload) => {
      if (payload.returnedTagsArray !== undefined) {
        tagStore.applyTags(payload.returnedTagsArray)
      } else {
        console.warn('Returned tags array is undefined')
      }
      modalStore.showEditTagsModal = false
    },

    notification: (payload) => {
      messageStore.push(payload.text, payload.color)
    },

    unauthorized: async () => {
      await redirectionStore.redirectionToLogin()
    },

    refreshTimestampToken: (payload) => {
      tokenStore.timestampToken = payload.timestampToken
    },

    refreshHashToken: (payload) => {
      tokenStore.hashTokenMap.set(payload.hash, payload.hashToken)
    }
  })

  const messageHandler = (e: MessageEvent) => {
    handler(e.data as ReturnType<(typeof fromDataWorker)[keyof typeof fromDataWorker]>)
  }

  dataWorker.addEventListener('message', messageHandler)
  workerHandlerMap.set(dataWorker, messageHandler)
}

/**
 * Removes the message listener associated with the given DataWorker.
 * Used for cleanup to prevent memory leaks when components unmount.
 *
 * @param dataWorker - The worker instance to clean up.
 */
export function removeHandleDataWorkerReturn(dataWorker: Worker) {
  const messageHandler = workerHandlerMap.get(dataWorker)
  if (messageHandler) {
    dataWorker.removeEventListener('message', messageHandler)
    workerHandlerMap.delete(dataWorker)
  }
}
