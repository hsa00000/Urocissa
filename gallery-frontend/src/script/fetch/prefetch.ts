import axios from 'axios'
import { Prefetch } from '../common/types'
import { prefetchSchema } from '../common/schemas'

export async function prefetch(
  filterJsonString: string | null,
  priorityId: string | undefined = 'default',
  reverse: string | undefined = 'false',
  locate: null | string = null
) {
  void priorityId
  void reverse
  const fetchUrl = `/get/prefetch?${locate !== null ? `locate=${locate}` : ''}`

  const axiosResponse = await axios.post<Prefetch>(fetchUrl, filterJsonString, {
    headers: {
      'Content-Type': 'application/json'
    }
  })

  const prefetch = prefetchSchema.parse(axiosResponse.data)

  return prefetch
}
