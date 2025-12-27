import axios from 'axios'
import { IsolationId, ScrollbarData } from '@type/types'
import { scrollbarDataSchema } from '@type/schemas'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { z } from 'zod'
import { useTokenStore } from '@/store/tokenStore'

export async function fetchScrollbar(isolationId: IsolationId) {
  const prefetchStore = usePrefetchStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const scrollbarStore = useScrollbarStore(isolationId)

  const timestamp = prefetchStore.timestamp
  if (timestamp === null) {
    console.error('timestamp is null, cannot fetch scrollbar')
    return
  }
  const timestampToken = tokenStore.timestampToken
  if (timestampToken === null) {
    console.error('timestampToken is null, cannot fetch scrollbar')
    return
  }
  const response = await axios.get<ScrollbarData[]>(`/get/get-scroll-bar?timestamp=${timestamp}`, {
    headers: {
      Authorization: `Bearer ${timestampToken}`
    }
  })
  const scrollbarDataArray = z.array(scrollbarDataSchema).parse(response.data)
  scrollbarStore.initialize(scrollbarDataArray)
}
