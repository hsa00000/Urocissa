import { beforeEach, describe, expect, it, vi } from 'vitest'
import axios from 'axios'
import { createPinia, setActivePinia } from 'pinia'
import { isHashTokenKnownFresh, registerHashTokenExpiration } from '@/script/utils/hashTokenExpiryRegistry'
import { useTokenStore } from './tokenStore'

function deferred<T>() {
  let resolve!: (value: T) => void
  const promise = new Promise<T>((resolvePromise) => {
    resolve = resolvePromise
  })
  return { promise, resolve }
}

describe('token store snapshot lifecycle', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.restoreAllMocks()
  })

  it('does not let an old renewal repopulate a cleared snapshot', async () => {
    const pending = deferred<{ data: { token: string } }>()
    vi.spyOn(axios, 'post').mockReturnValue(pending.promise)
    const store = useTokenStore('mainId')
    store.timestampToken = 'old-timestamp-token'
    store.hashTokenMap.set('hash', 'old-hash-token')
    registerHashTokenExpiration('mainId', 'hash', 200)

    const renewal = store._updateTimestampToken()
    store.clearAll()
    pending.resolve({ data: { token: 'renewed-timestamp-token' } })
    await renewal

    expect(store.timestampToken).toBeNull()
    expect(store.hashTokenMap.size).toBe(0)
    expect(isHashTokenKnownFresh('mainId', 'hash', 100)).toBe(false)
  })
})
