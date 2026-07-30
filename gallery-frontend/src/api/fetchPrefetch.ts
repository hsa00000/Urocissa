import axios from 'axios'
import { Prefetch, PrefetchReturn, type GallerySortOrder } from '@type/types'
import { prefetchReturnSchema } from '@type/schemas'

export async function prefetch(
  filterJsonString: string | null,
  priorityId: string | undefined = 'default',
  sortOrder: GallerySortOrder = 'descending',
  locate: null | string = null
): Promise<PrefetchReturn> {
  void priorityId

  const axiosResponse = await axios.post<Prefetch>('/get/prefetch', filterJsonString, {
    params: {
      locate: locate ?? undefined,
      sort: sortOrder
    },
    headers: {
      'Content-Type': 'application/json'
    }
  })

  const prefetchReturn = prefetchReturnSchema.parse(axiosResponse.data)

  return prefetchReturn
}
