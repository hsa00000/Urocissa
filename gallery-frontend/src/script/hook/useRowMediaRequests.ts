import { computed, onScopeDispose, watch, type Ref } from 'vue'
import type { EnrichedUnifiedData, IsolationId, Row } from '@type/types'
import { useConstStore } from '@/store/constStore'
import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useQueueStore } from '@/store/queueStore'
import { useShareStore } from '@/store/shareStore'
import { useTokenStore } from '@/store/tokenStore'
import { useWorkerStore } from '@/store/workerStore'
import { getArrayValue } from '@utils/getter'
import { dispatchImageRequestsInParallel } from '@/script/utils/parallelImageRequests'
import { isHashTokenKnownFresh } from '@/script/utils/hashTokenExpiryRegistry'

interface PendingImageRequest {
  abstractData: EnrichedUnifiedData
  index: number
  displayWidth: number
  displayHeight: number
}

interface ImageSource {
  hash: string
  albumMode: boolean
}

function getImageSource(abstractData: EnrichedUnifiedData): ImageSource | null {
  if (abstractData.type === 'image' || abstractData.type === 'video') {
    return { hash: abstractData.id, albumMode: false }
  }

  if (abstractData.cover !== null) {
    return { hash: abstractData.cover, albumMode: true }
  }

  return null
}

export function useRowMediaRequests(
  row: Readonly<Ref<Row>>,
  isolationId: IsolationId,
  imagesDisabled: Readonly<Ref<boolean>>
) {
  const constStore = useConstStore('mainId')
  const dataStore = useDataStore(isolationId)
  const imgStore = useImgStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const queueStore = useQueueStore(isolationId)
  const shareStore = useShareStore('mainId')
  const tokenStore = useTokenStore(isolationId)
  const workerStore = useWorkerStore(isolationId)
  let generation = 0

  watch(
    () => [row.value.start, row.value.end, prefetchStore.timestamp] as const,
    () => {
      generation += 1
    },
    { immediate: true, flush: 'sync' }
  )

  const pendingImageRequests = computed(() => {
    void prefetchStore.timestamp
    if (imagesDisabled.value) {
      return []
    }

    return row.value.displayElements
      .map((displayElement, subIndex): PendingImageRequest | null => {
        const index = row.value.start + subIndex
        const abstractData = dataStore.data.get(index)

        if (
          abstractData === undefined ||
          queueStore.img.has(index) ||
          imgStore.imgUrl.has(index)
        ) {
          return null
        }

        return {
          abstractData,
          index,
          displayWidth: displayElement.displayWidth,
          displayHeight: displayElement.displayHeight
        }
      })
      .filter((request): request is PendingImageRequest => request !== null)
  })

  watch(
    pendingImageRequests,
    (pending) => {
      if (pending.length === 0) {
        return
      }

      pending.forEach((request) => queueStore.img.add(request.index))
      const requestGeneration = generation
      dispatchRequests(pending, requestGeneration).catch((error: unknown) => {
        console.error('row image request failed:', error)
      })
    },
    { immediate: true }
  )

  async function dispatchRequests(
    pending: PendingImageRequest[],
    requestGeneration: number
  ): Promise<void> {
    await tokenStore.refreshTimestampTokenIfExpired()
    if (requestGeneration !== generation) {
      return
    }

    const timestampToken = tokenStore.timestampToken
    if (timestampToken === null) {
      throw new Error('timestampToken is null after refresh')
    }

    const postToImgWorkerList = workerStore.postToImgWorkerList
    if (postToImgWorkerList === undefined) {
      console.error('workerStore.postToImgWorkerList is undefined')
      return
    }

    const results = await dispatchImageRequestsInParallel(pending, async (request) => {
      if (
        requestGeneration !== generation ||
        imgStore.imgUrl.has(request.index)
      ) {
        return
      }

      const source = getImageSource(request.abstractData)
      if (source === null) {
        return
      }

      if (!isHashTokenKnownFresh(isolationId, source.hash)) {
        await tokenStore.refreshHashTokenIfExpired(source.hash)
      }
      if (requestGeneration !== generation) {
        return
      }

      const hashToken = tokenStore.hashTokenMap.get(source.hash)
      if (hashToken === undefined) {
        throw new Error(`hashToken is undefined after refresh for hash: ${source.hash}`)
      }

      const workerIndex = request.index % constStore.concurrencyNumber
      getArrayValue(postToImgWorkerList, workerIndex).processSmallImage({
        index: request.index,
        hash: source.hash,
        width: request.displayWidth,
        height: request.displayHeight,
        devicePixelRatio: window.devicePixelRatio,
        ...(source.albumMode ? { albumMode: true } : {}),
        albumId: shareStore.albumId,
        shareId: shareStore.shareId,
        password: shareStore.password,
        timestampToken,
        hashToken,
        cacheVersion: request.abstractData.cacheVersion
      })
    })

    results.forEach((result, resultIndex) => {
      if (result.status === 'rejected') {
        const request = pending[resultIndex]
        const index = request?.index ?? 'unknown'
        console.error(`small image request failed for index ${index}:`, result.reason)
      }
    })
  }

  onScopeDispose(() => {
    generation += 1
  })

  return { dataStore, imgStore }
}
