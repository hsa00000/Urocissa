import axios from 'axios'
import { IsolationId, ScrollbarData } from '@/script/common/types'
import { z } from 'zod'
import { scrollbarDataSchema } from '../common/schemas'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { getToken } from '@/indexedDb/token'

export async function fetchScrollbar(isolationId: IsolationId) {
  const prefetchStore = usePrefetchStore(isolationId)

  const scrollbarStore = useScrollbarStore(isolationId)

  const timestamp = prefetchStore.timestamp
  const timestampToken = await getToken()
  const response = await axios.get<ScrollbarData[]>(`/get/get-scroll-bar?timestamp=${timestamp}`, {
    headers: {
      Authorization: `Bearer ${timestampToken}`
    }
  })
  const scrollbarDataArray = z.array(scrollbarDataSchema).parse(response.data)

  console.log('payload.scrollbarDataArray is ', scrollbarDataArray)
  scrollbarStore.initialize(scrollbarDataArray)
}
