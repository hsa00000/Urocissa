import { beforeEach, describe, expect, it, vi } from 'vitest'
import axios from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { fetchScrollbar } from './fetchScrollbar'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollbarStore } from '@/store/scrollbarStore'
import { useTokenStore } from '@/store/tokenStore'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

describe('scrollbar snapshot generation', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('does not let an older timestamp overwrite the current scrollbar', async () => {
    const older = deferred<{ data: unknown }>()
    const newer = deferred<{ data: unknown }>()
    vi.spyOn(axios, 'get')
      .mockReturnValueOnce(older.promise)
      .mockReturnValueOnce(newer.promise)

    const prefetchStore = usePrefetchStore('subId')
    const tokenStore = useTokenStore('subId')
    prefetchStore.timestamp = 1
    tokenStore.timestampToken = 'token-1'
    const olderRequest = fetchScrollbar('subId')

    prefetchStore.timestamp = 2
    tokenStore.timestampToken = 'token-2'
    const newerRequest = fetchScrollbar('subId')
    newer.resolve({ data: [{ year: 2026, month: 8, index: 20 }] })
    await newerRequest

    older.resolve({ data: [{ year: 2025, month: 1, index: 0 }] })
    await olderRequest

    expect(useScrollbarStore('subId').scrollbarDataArray).toEqual([
      { year: 2026, month: 8, index: 20 }
    ])
  })
})
