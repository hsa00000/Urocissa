import axios from 'axios'
import { Prefetch, PrefetchReturn } from '../common/types'
import { prefetchReturnSchema } from '../common/schemas'

export async function prefetch(
  filterJsonString: string | null,
  priorityId: string | undefined = 'default',
  reverse: string | undefined = 'false',
  locate: null | string = null
): Promise<PrefetchReturn> {
  void priorityId
  void reverse
  const fetchUrl = `/get/prefetch?${locate !== null ? `locate=${locate}` : ''}`

  const axiosResponse = await axios.post<Prefetch>(fetchUrl, filterJsonString, {
    headers: {
      'Content-Type': 'application/json'
    }
  })

  const prefetchReturn = prefetchReturnSchema.parse(axiosResponse.data)

  return prefetchReturn
}
