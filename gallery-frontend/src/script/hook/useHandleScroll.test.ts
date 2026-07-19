import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { shallowRef } from 'vue'
import { useConfigStore } from '@/store/configStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { applyWheelDeltaToPhysicalBuffer, handleScroll } from './useHandleScroll'

function createContainer(scrollTop: number): HTMLElement {
  return { scrollTop } as HTMLElement
}

describe('compensated virtual scroll behavior', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  it.each([
    {
      name: 'moves within bounds',
      totalHeight: 1000,
      windowHeight: 100,
      virtualTop: 100,
      physicalTop: 120,
      expectedVirtualTop: 120
    },
    {
      name: 'clamps at the upper edge',
      totalHeight: 1000,
      windowHeight: 100,
      virtualTop: 5,
      physicalTop: 90,
      expectedVirtualTop: 0
    },
    {
      name: 'clamps at the lower edge',
      totalHeight: 1000,
      windowHeight: 100,
      virtualTop: 890,
      physicalTop: 120,
      expectedVirtualTop: 896
    },
    {
      name: 'keeps short content at zero',
      totalHeight: 50,
      windowHeight: 100,
      virtualTop: 10,
      physicalTop: 120,
      expectedVirtualTop: 0
    }
  ])('$name while restoring the physical anchor', (fixture) => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = fixture.totalHeight
    scrollTopStore.scrollTop = fixture.virtualTop
    const container = createContainer(fixture.physicalTop)
    const imageContainerRef = shallowRef<HTMLElement | null>(container)
    const lastScrollTop = shallowRef(100)
    const stopScroll = shallowRef(false)
    const windowHeight = shallowRef(fixture.windowHeight)
    const throttledHandleScroll = handleScroll(
      imageContainerRef,
      lastScrollTop,
      stopScroll,
      windowHeight,
      'tempId'
    )

    throttledHandleScroll()

    expect(scrollTopStore.scrollTop).toBe(fixture.expectedVirtualTop)
    expect(container.scrollTop).toBe(100)
    expect(lastScrollTop.value).toBe(100)
    throttledHandleScroll.cancel()
  })

  it('preserves the mobile stop-scroll window for short content', () => {
    useConfigStore('mainId').isMobile = true
    const prefetchStore = usePrefetchStore('tempId')
    prefetchStore.totalHeight = 50
    const container = createContainer(120)
    const stopScroll = shallowRef(false)
    const throttledHandleScroll = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      stopScroll,
      shallowRef(100),
      'tempId'
    )

    throttledHandleScroll()
    expect(stopScroll.value).toBe(true)
    vi.advanceTimersByTime(99)
    expect(stopScroll.value).toBe(true)
    vi.advanceTimersByTime(1)
    expect(stopScroll.value).toBe(false)
    throttledHandleScroll.cancel()
  })

  it('writes a cancelable pixel wheel delta to the physical buffer exactly once', () => {
    const container = createContainer(200000)
    let prevented = false
    const event = {
      cancelable: true,
      ctrlKey: false,
      get defaultPrevented() {
        return prevented
      },
      deltaMode: 0,
      deltaY: 100,
      preventDefault() {
        prevented = true
      }
    }

    expect(applyWheelDeltaToPhysicalBuffer(container, event)).toBe(true)
    expect(prevented).toBe(true)
    expect(container.scrollTop).toBe(200100)
  })

  it.each([
    { name: 'non-cancelable', cancelable: false, ctrlKey: false, deltaMode: 0, deltaY: 100 },
    { name: 'zoom gesture', cancelable: true, ctrlKey: true, deltaMode: 0, deltaY: 100 },
    { name: 'line mode', cancelable: true, ctrlKey: false, deltaMode: 1, deltaY: 3 },
    { name: 'horizontal-only', cancelable: true, ctrlKey: false, deltaMode: 0, deltaY: 0 }
  ])('leaves $name wheel input to the browser', (fixture) => {
    const container = createContainer(200000)
    const preventDefault = vi.fn()

    expect(
      applyWheelDeltaToPhysicalBuffer(container, {
        ...fixture,
        defaultPrevented: false,
        preventDefault
      })
    ).toBe(false)
    expect(preventDefault).not.toHaveBeenCalled()
    expect(container.scrollTop).toBe(200000)
  })
})
