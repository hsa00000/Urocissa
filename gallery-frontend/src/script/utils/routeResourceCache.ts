import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { useAlbumStore } from '@/store/albumStore'
import { useRouteResourceStore } from '@/store/routeResourceStore'
import { createSelectionMatcher } from '@/type/selection'
import type { SelectionDescriptor } from '@/type/selection'
import type {
  CollectionIsolationId,
  EnrichedUnifiedData,
  IsolationId,
  RouteResourceIsolationId
} from '@/type/types'

export const allResourceIsolationIds = [
  'mainId',
  'subId',
  'tempId',
  'shareId',
  'detailId',
  'subDetailId'
] as const satisfies readonly IsolationId[]

export function hasCachedResource(isolationId: IsolationId, resourceId: string): boolean {
  const dataStore = useDataStore(isolationId)
  const index = dataStore.hashMapData.get(resourceId)
  return index !== undefined && dataStore.data.has(index)
}

export function collectionIsolationForResource(
  isolationId: IsolationId
): CollectionIsolationId {
  if (isolationId === 'detailId') return 'mainId'
  if (isolationId === 'subDetailId') return 'subId'
  return isolationId
}

export function isRouteResourceIsolation(
  isolationId: IsolationId
): isolationId is RouteResourceIsolationId {
  return isolationId === 'detailId' || isolationId === 'subDetailId'
}

export function updateCachedResource(
  resourceId: string,
  update: (data: EnrichedUnifiedData, isolationId: IsolationId, index: number) => void
): void {
  for (const isolationId of allResourceIsolationIds) {
    const dataStore = useDataStore(isolationId)
    const index = dataStore.hashMapData.get(resourceId)
    if (index === undefined) continue
    const data = dataStore.data.get(index)
    if (data !== undefined) update(data, isolationId, index)
  }
}

export function selectedCachedResourceIds(
  isolationId: IsolationId,
  selection: SelectionDescriptor
): string[] {
  const isSelected = createSelectionMatcher(selection)
  const ids = new Set<string>()
  for (const [index, data] of useDataStore(isolationId).data) {
    if (isSelected(index)) ids.add(data.id)
  }
  return [...ids]
}

export function clearCachedResource(resourceId: string): Set<CollectionIsolationId> {
  const affectedCollections = new Set<CollectionIsolationId>()
  for (const isolationId of allResourceIsolationIds) {
    if (isolationId === 'detailId' || isolationId === 'subDetailId') {
      const routeStore = useRouteResourceStore(isolationId)
      if (routeStore.requestedId === resourceId) {
        routeStore.clear()
        continue
      }
    }

    const dataStore = useDataStore(isolationId)
    const index = dataStore.hashMapData.get(resourceId)
    if (index === undefined) continue
    if (!isRouteResourceIsolation(isolationId)) affectedCollections.add(isolationId)
    dataStore.hashMapData.delete(resourceId)
    dataStore.data.delete(index)
    dataStore.batchFetched.delete(index)
    const imgStore = useImgStore(isolationId)
    imgStore.imgOriginal.delete(index)
    imgStore.imgUrl.delete(index)
  }

  for (const isolationId of ['mainId', 'subId', 'tempId', 'shareId'] as const) {
    useAlbumStore(isolationId).albums.delete(resourceId)
  }

  return affectedCollections
}
