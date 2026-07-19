import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { shallowRef } from 'vue'
import { useConfigStore } from '@/store/configStore'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { handleScroll } from './useHandleScroll'

function createContainer(scrollTop: number): HTMLElement {
  return { scrollTop, onscrollend: null } as unknown as HTMLElement
}

describe('native compensated virtual scroll transactions', () => {
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
      expectedPhysicalDuringScroll: 120,
      expectedVirtualTop: 120
    },
    {
      name: 'clamps at the upper edge',
      totalHeight: 1000,
      windowHeight: 100,
      virtualTop: 5,
      physicalTop: 90,
      expectedPhysicalDuringScroll: 95,
      expectedVirtualTop: 0
    },
    {
      name: 'clamps at the lower edge',
      totalHeight: 1000,
      windowHeight: 100,
      virtualTop: 890,
      physicalTop: 120,
      expectedPhysicalDuringScroll: 106,
      expectedVirtualTop: 896
    },
    {
      name: 'keeps short content at zero',
      totalHeight: 50,
      windowHeight: 100,
      virtualTop: 10,
      physicalTop: 120,
      expectedPhysicalDuringScroll: 100,
      expectedVirtualTop: 0
    }
  ])('$name while preserving native physical movement until scrollend', async (fixture) => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = fixture.totalHeight
    scrollTopStore.scrollTop = fixture.virtualTop
    const container = createContainer(fixture.physicalTop)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      shallowRef(false),
      shallowRef(fixture.windowHeight),
      'tempId'
    )

    controller.onScroll()

    expect(container.scrollTop).toBe(fixture.expectedPhysicalDuringScroll)
    expect(controller.effectiveScrollTop.value).toBe(fixture.expectedVirtualTop)
    if (fixture.totalHeight > fixture.windowHeight) {
      expect(scrollTopStore.scrollTop).toBe(fixture.virtualTop)
    }

    await controller.onScrollEnd()

    expect(scrollTopStore.scrollTop).toBe(fixture.expectedVirtualTop)
    expect(container.scrollTop).toBe(100)
    controller.cancel()
  })

  it('does not write the physical position during an in-bounds native animation', () => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = 1000
    scrollTopStore.scrollTop = 100
    const container = createContainer(101)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      shallowRef(false),
      shallowRef(100),
      'tempId'
    )

    controller.onScroll()

    expect(container.scrollTop).toBe(101)
    expect(scrollTopStore.scrollTop).toBe(100)
    expect(controller.effectiveScrollTop.value).toBe(101)
    controller.cancel()
  })

  it('coalesces repeated native scroll observations without committing early', async () => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = 1000
    scrollTopStore.scrollTop = 100
    const container = createContainer(101)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      shallowRef(false),
      shallowRef(100),
      'tempId'
    )

    controller.onScroll()
    container.scrollTop = 105
    controller.onScroll()

    expect(scrollTopStore.scrollTop).toBe(100)
    expect(controller.effectiveScrollTop.value).toBe(101)

    vi.advanceTimersByTime(100)
    expect(controller.effectiveScrollTop.value).toBe(105)
    expect(container.scrollTop).toBe(105)

    await controller.onScrollEnd()
    expect(scrollTopStore.scrollTop).toBe(105)
    expect(container.scrollTop).toBe(100)
    controller.cancel()
  })

  it('keeps transient movement when committed geometry receives an offset shift', () => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = 1000
    scrollTopStore.scrollTop = 100
    const container = createContainer(120)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      shallowRef(false),
      shallowRef(100),
      'tempId'
    )

    controller.onScroll()
    scrollTopStore.scrollTop += 25

    expect(container.scrollTop).toBe(120)
    expect(controller.effectiveScrollTop.value).toBe(145)
    controller.cancel()
  })

  it('abandons transient movement before an external virtual jump', () => {
    const prefetchStore = usePrefetchStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    prefetchStore.totalHeight = 1000
    scrollTopStore.scrollTop = 100
    const container = createContainer(150)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      shallowRef(false),
      shallowRef(100),
      'tempId'
    )

    controller.onScroll()
    expect(controller.effectiveScrollTop.value).toBe(150)

    controller.resetPhysicalAnchor()

    expect(scrollTopStore.scrollTop).toBe(100)
    expect(controller.effectiveScrollTop.value).toBe(100)
    expect(container.scrollTop).toBe(100)
    controller.cancel()
  })

  it('preserves the mobile stop-scroll window for short content', () => {
    useConfigStore('mainId').isMobile = true
    const prefetchStore = usePrefetchStore('tempId')
    prefetchStore.totalHeight = 50
    const container = createContainer(120)
    const stopScroll = shallowRef(false)
    const controller = handleScroll(
      shallowRef<HTMLElement | null>(container),
      shallowRef(100),
      stopScroll,
      shallowRef(100),
      'tempId'
    )

    controller.onScroll()
    expect(stopScroll.value).toBe(true)
    vi.advanceTimersByTime(99)
    expect(stopScroll.value).toBe(true)
    vi.advanceTimersByTime(1)
    expect(stopScroll.value).toBe(false)
    controller.cancel()
  })
})
