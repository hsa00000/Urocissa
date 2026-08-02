import { useDataStore } from '@/store/dataStore'
import { useImgStore } from '@/store/imgStore'
import { useAlbumStore } from '@/store/albumStore'
import { useRouteResourceStore } from '@/store/routeResourceStore'
import { createSelectionMatcher } from '@/type/selection'
import type { SelectionDescriptor } from '@/type/selection'
import type { EnrichedUnifiedData, IsolationId } from '@/type/types'

export const allResourceIsolationIds = [
  'mainId',
  'subId',
  'tempId',
  'shareId',
  'detailId',
  'subDetailId'
] as const satisfies readonly IsolationId[]

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

export function clearCachedResource(resourceId: string): void {
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
}
