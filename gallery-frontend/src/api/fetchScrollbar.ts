import axios from 'axios'
import { IsolationId, ScrollbarData } from '@type/types'
import { scrollbarDataSchema } from '@type/schemas'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { z } from 'zod'
import { useTokenStore } from '@/store/tokenStore'

interface PendingScrollbarRequest {
  timestamp: number
  promise: Promise<void>
}

const pendingRequests = new Map<IsolationId, PendingScrollbarRequest>()

export async function fetchScrollbar(isolationId: IsolationId) {
  const prefetchStore = usePrefetchStore(isolationId)
  const tokenStore = useTokenStore(isolationId)
  const scrollbarStore = useScrollbarStore(isolationId)

  const timestamp = prefetchStore.timestamp
  if (timestamp === null) {
    console.error('timestamp is null, cannot fetch scrollbar')
    return
  }
  const pending = pendingRequests.get(isolationId)
  if (pending?.timestamp === timestamp) return pending.promise
  const timestampToken = tokenStore.timestampToken
  if (timestampToken === null) {
    console.error('timestampToken is null, cannot fetch scrollbar')
    return
  }
  const request = (async () => {
    const response = await axios.get<ScrollbarData[]>(
      `/get/get-scroll-bar?timestamp=${timestamp}`,
      {
        headers: {
          Authorization: `Bearer ${timestampToken}`
        }
      }
    )
    const scrollbarDataArray = z.array(scrollbarDataSchema).parse(response.data)
    // A newer collection snapshot may have replaced this timestamp while the
    // request was in flight. Never attach old scrollbar boundaries to it.
    if (prefetchStore.timestamp !== timestamp) return
    scrollbarStore.initialize(scrollbarDataArray)
  })()
  pendingRequests.set(isolationId, { timestamp, promise: request })

  try {
    await request
  } finally {
    if (pendingRequests.get(isolationId)?.promise === request) pendingRequests.delete(isolationId)
  }
}
