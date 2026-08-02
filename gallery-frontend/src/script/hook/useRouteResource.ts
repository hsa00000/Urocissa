import { computed, onBeforeUnmount, toValue, watch } from 'vue'
import type { MaybeRefOrGetter } from 'vue'
import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { useRouteResourceStore } from '@/store/routeResourceStore'
import type {
  EnrichedUnifiedData,
  IsolationId,
  RouteResourceIsolationId
} from '@/type/types'

export type RouteResourceExpectedType = EnrichedUnifiedData['type']

export interface ResolvedRouteResource {
  isolationId: IsolationId
  index: number
  data: EnrichedUnifiedData
}

export function useRouteResourceLoader(
  resourceId: MaybeRefOrGetter<string>,
  isolationId: RouteResourceIsolationId,
  enabled: MaybeRefOrGetter<boolean> = true
) {
  const store = useRouteResourceStore(isolationId)

  watch(
    [() => toValue(resourceId), () => toValue(enabled)],
    ([id, isEnabled]) => {
      if (!isEnabled || id === '') {
        store.clear()
        return
      }
      void store.load(id)
    },
    { immediate: true }
  )

  onBeforeUnmount(() => {
    store.clear()
  })

  return {
    retry: () => store.load(toValue(resourceId), true)
  }
}

export function useResolvedRouteResource(
  resourceId: MaybeRefOrGetter<string>,
  collectionIsolationId: IsolationId,
  directIsolationId?: RouteResourceIsolationId,
  expectedTypes?: readonly RouteResourceExpectedType[]
) {
  const collectionStore = useDataStore(collectionIsolationId)
  const directDataStore = directIsolationId ? useDataStore(directIsolationId) : undefined
  const directStore = directIsolationId ? useRouteResourceStore(directIsolationId) : undefined

  const resource = computed<ResolvedRouteResource | undefined>(() => {
    const id = toValue(resourceId)
    const collectionIndex = collectionStore.hashMapData.get(id)
    const collectionData =
      collectionIndex === undefined ? undefined : collectionStore.data.get(collectionIndex)
    if (collectionIndex !== undefined && collectionData !== undefined) {
      return { isolationId: collectionIsolationId, index: collectionIndex, data: collectionData }
    }

    if (directIsolationId === undefined || directDataStore === undefined) return undefined
    const directIndex = directDataStore.hashMapData.get(id)
    const directData = directIndex === undefined ? undefined : directDataStore.data.get(directIndex)
    if (directIndex === undefined || directData === undefined) return undefined
    return { isolationId: directIsolationId, index: directIndex, data: directData }
  })

  const wrongType = computed(
    () =>
      resource.value !== undefined &&
      expectedTypes !== undefined &&
      !expectedTypes.includes(resource.value.data.type)
  )

  const status = computed(() => {
    if (wrongType.value) return 'wrong-type' as const
    if (resource.value !== undefined) return 'ready' as const
    if (directStore === undefined) return 'loading' as const
    if (directStore.requestedId !== toValue(resourceId)) return 'loading' as const
    return directStore.status
  })

  const errorMessage = computed(() => directStore?.errorMessage ?? null)

  // A direct snapshot can render before the background collection locates the
  // same item. Preserve the already-decoded media before collection priority
  // remounts the viewer at its real index, so the handoff has no blank frame.
  watch(
    resource,
    (current, previous) => {
      if (
        current === undefined ||
        previous === undefined ||
        directIsolationId === undefined ||
        previous.isolationId !== directIsolationId ||
        current.isolationId !== collectionIsolationId ||
        previous.data.id !== current.data.id
      ) {
        return
      }

      const directImages = useImgStore(directIsolationId)
      const collectionImages = useImgStore(collectionIsolationId)
      const original = directImages.imgOriginal.get(previous.index)
      const thumbnail = directImages.imgUrl.get(previous.index)

      if (original !== undefined && !collectionImages.imgOriginal.has(current.index)) {
        collectionImages.imgOriginal.set(current.index, original)
      }
      if (thumbnail !== undefined && !collectionImages.imgUrl.has(current.index)) {
        collectionImages.imgUrl.set(current.index, thumbnail)
      }
    },
    { flush: 'pre' }
  )

  return {
    resource,
    status,
    errorMessage,
    retry: () => directStore?.load(toValue(resourceId), true)
  }
}
