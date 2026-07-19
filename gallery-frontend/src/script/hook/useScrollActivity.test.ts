import { afterEach, describe, expect, it, vi } from 'vitest'
import { effectScope, nextTick, shallowRef } from 'vue'
import {
  useRowScrollActivity,
  useScrollActivity,
  type ScrollActivityContext
} from './useScrollActivity'

describe('shared scroll activity', () => {
  afterEach(() => {
    vi.useRealTimers()
  })

  it('uses one idle timer and preserves row mount-generation behavior', async () => {
    vi.useFakeTimers()
    const scrollTop = shallowRef(0)
    const scope = effectScope()
    let activity: ScrollActivityContext | undefined
    let existingRowIsScrolling: ReturnType<typeof useRowScrollActivity> | undefined

    scope.run(() => {
      activity = useScrollActivity(() => scrollTop.value)
      existingRowIsScrolling = useRowScrollActivity(activity)
    })
    if (activity === undefined || existingRowIsScrolling === undefined) {
      throw new Error('scroll activity fixture was not initialized')
    }

    expect(existingRowIsScrolling.value).toBe(false)
    scrollTop.value = 12
    await nextTick()
    expect(activity.generation.value).toBe(1)
    expect(existingRowIsScrolling.value).toBe(true)
    expect(vi.getTimerCount()).toBe(1)

    let newlyMountedRowIsScrolling: ReturnType<typeof useRowScrollActivity> | undefined
    scope.run(() => {
      newlyMountedRowIsScrolling = useRowScrollActivity(activity)
    })
    if (newlyMountedRowIsScrolling === undefined) {
      throw new Error('new row fixture was not initialized')
    }
    expect(newlyMountedRowIsScrolling.value).toBe(false)

    scrollTop.value = 24
    await nextTick()
    expect(existingRowIsScrolling.value).toBe(true)
    expect(newlyMountedRowIsScrolling.value).toBe(true)
    expect(vi.getTimerCount()).toBe(1)

    vi.advanceTimersByTime(100)
    expect(existingRowIsScrolling.value).toBe(false)
    expect(newlyMountedRowIsScrolling.value).toBe(false)

    scrollTop.value = 36
    await nextTick()
    expect(vi.getTimerCount()).toBe(1)
    scope.stop()
    expect(vi.getTimerCount()).toBe(0)
  })
})
