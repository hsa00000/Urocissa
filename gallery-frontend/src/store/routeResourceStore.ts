import axios from 'axios'
import { defineStore } from 'pinia'
import { fetchRouteResource } from '@/api/fetchRouteResource'
import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { useInitializedStore } from '@/store/initializedStore'
import { useOptimisticStore } from '@/store/optimisticUpateStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useQueueStore } from '@/store/queueStore'
import { useTokenStore } from '@/store/tokenStore'
import { useWorkerStore } from '@/store/workerStore'
import { enrichWithThumbhash } from '@/script/utils/createData'
import { serverErrorSchema } from '@/type/schemas'
import {
  readJwtExpiration,
  registerHashTokenExpiration,
  resetHashTokenExpirations
} from '@/script/utils/hashTokenExpiryRegistry'
import type { RouteResourceIsolationId, RouteResourceSnapshot } from '@/type/types'

export type RouteResourceStatus =
  | 'idle'
  | 'loading'
  | 'ready'
  | 'invalid'
  | 'not-found'
  | 'error'

interface PendingRequest {
  id: string
  promise: Promise<void>
  controller: AbortController
}

const pendingByIsolation = new Map<RouteResourceIsolationId, PendingRequest>()

function routeResourceErrorMessage(error: unknown): string {
  if (axios.isAxiosError(error)) {
    const parsed = serverErrorSchema.safeParse(error.response?.data)
    if (parsed.success) {
      const message = parsed.data.message ?? parsed.data.error
      if (message !== undefined && message !== '') return message
    }
    return error.message
  }
  return error instanceof Error ? error.message : 'Unknown error occurred'
}

function clearHydratedStores(isolationId: RouteResourceIsolationId): void {
  useInitializedStore(isolationId).initialized = false
  useDataStore(isolationId).clearAll()
  usePrefetchStore(isolationId).clearAll()
  useTokenStore(isolationId).timestampToken = null
  useTokenStore(isolationId).hashTokenMap.clear()
  useImgStore(isolationId).clearAll()
  useQueueStore(isolationId).clearAll()
  useOptimisticStore(isolationId).clearAll()
  resetHashTokenExpirations(isolationId)
}

function terminateHydrationWorkers(isolationId: RouteResourceIsolationId): void {
  const workerStore = useWorkerStore(isolationId)
  if (workerStore.worker !== null || workerStore.imgWorker.length > 0) {
    workerStore.terminateWorker()
  }
}

function hydrateSnapshot(
  isolationId: RouteResourceIsolationId,
  snapshot: RouteResourceSnapshot
): void {
  const dataStore = useDataStore(isolationId)
  const prefetchStore = usePrefetchStore(isolationId)
  const initializedStore = useInitializedStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const item = enrichWithThumbhash({
    ...snapshot.data.abstractData,
    timestamp: snapshot.data.timestamp
  })

  dataStore.data.set(0, item)
  dataStore.hashMapData.set(item.id, 0)
  dataStore.batchFetched.set(0, true)

  prefetchStore.timestamp = snapshot.prefetch.timestamp
  prefetchStore.calculateLength(snapshot.prefetch.dataLength)
  prefetchStore.locateTo = snapshot.prefetch.locateTo
  tokenStore.timestampToken = snapshot.token

  const tokenHash = item.type === 'album' ? item.cover : item.id
  if (tokenHash !== null && snapshot.data.token !== '') {
    tokenStore.hashTokenMap.set(tokenHash, snapshot.data.token)
    registerHashTokenExpiration(
      isolationId,
      tokenHash,
      readJwtExpiration(snapshot.data.token)
    )
  }

  const workerStore = useWorkerStore(isolationId)
  if (workerStore.worker === null && workerStore.imgWorker.length === 0) {
    workerStore.initializeWorker(isolationId)
  }
  initializedStore.initialized = true
}

export const useRouteResourceStore = (isolationId: RouteResourceIsolationId) =>
  defineStore(`routeResourceStore${isolationId}`, {
    state: (): {
      requestedId: string | null
      status: RouteResourceStatus
      errorMessage: string | null
      generation: number
    } => ({
      requestedId: null,
      status: 'idle',
      errorMessage: null,
      generation: 0
    }),
    actions: {
      async load(resourceId: string, force = false): Promise<void> {
        const existing = pendingByIsolation.get(isolationId)
        if (!force && existing?.id === resourceId) return existing.promise
        if (!force && this.requestedId === resourceId && this.status === 'ready') return

        existing?.controller.abort()
        this.generation += 1
        const generation = this.generation
        const controller = new AbortController()
        // Worker replies are keyed only by collection index. A late reply for
        // the previous route resource would otherwise populate index 0 after
        // the next ID has taken ownership of this isolation.
        terminateHydrationWorkers(isolationId)
        clearHydratedStores(isolationId)
        this.requestedId = resourceId
        this.status = 'loading'
        this.errorMessage = null

        const promise = (async () => {
          try {
            const snapshot = await fetchRouteResource(resourceId, controller.signal)
            if (generation !== this.generation || this.requestedId !== resourceId) return
            hydrateSnapshot(isolationId, snapshot)
            this.status = 'ready'
          } catch (error: unknown) {
            if (generation !== this.generation || axios.isCancel(error)) return
            clearHydratedStores(isolationId)
            const status = axios.isAxiosError(error) ? error.response?.status : undefined
            if (status === 400) {
              this.status = 'invalid'
              this.errorMessage = routeResourceErrorMessage(error)
            } else if (status === 404) {
              this.status = 'not-found'
              this.errorMessage = routeResourceErrorMessage(error)
            } else {
              this.status = 'error'
              this.errorMessage =
                axios.isAxiosError(error) && error.code === 'ECONNABORTED'
                  ? 'The item request timed out. Please try again.'
                  : routeResourceErrorMessage(error)
            }
          } finally {
            const pending = pendingByIsolation.get(isolationId)
            if (pending?.controller === controller) pendingByIsolation.delete(isolationId)
          }
        })()

        pendingByIsolation.set(isolationId, { id: resourceId, promise, controller })
        return promise
      },
      cancel(): void {
        pendingByIsolation.get(isolationId)?.controller.abort()
        pendingByIsolation.delete(isolationId)
        this.generation += 1
      },
      clear(): void {
        this.cancel()
        terminateHydrationWorkers(isolationId)
        clearHydratedStores(isolationId)
        this.requestedId = null
        this.status = 'idle'
        this.errorMessage = null
      }
    }
  })()
