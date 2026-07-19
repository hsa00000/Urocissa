import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { useOffsetStore } from './offsetStore'

describe('offset store accumulation', () => {
  beforeEach(() => setActivePinia(createPinia()))

  it('sums only preceding row offsets without scheduling macrotasks', () => {
    const store = useOffsetStore('tempId')
    store.offset.set(0, 12)
    store.offset.set(2, -4)
    store.offset.set(5, 9)
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout')

    expect(store.accumulatedOffset(0)).toBe(0)
    expect(store.accumulatedOffset(2)).toBe(12)
    expect(store.accumulatedOffset(5)).toBe(8)
    expect(store.accumulatedOffset(6)).toBe(17)
    expect(setTimeoutSpy).not.toHaveBeenCalled()

    setTimeoutSpy.mockRestore()
  })
})
