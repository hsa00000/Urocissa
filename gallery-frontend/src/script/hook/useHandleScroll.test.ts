import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { nextTick, shallowRef } from 'vue'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useScrollTopStore } from '@/store/scrollTopStore'
import {
  handleScroll,
  selectHybridScrollMode,
  type HybridScrollController
} from './useHandleScroll'

interface TestContainer extends HTMLElement {
  readonly writes: number[]
  moveTo: (physicalTop: number) => void
  setScrollHeight: (scrollHeight: number) => void
}

function createContainer({
  scrollTop = 0,
  scrollHeight = 600_000,
  clientHeight = 100,
  supportsScrollEnd = true
}: {
  scrollTop?: number
  scrollHeight?: number
  clientHeight?: number
  supportsScrollEnd?: boolean
} = {}): TestContainer {
  let physicalTop = scrollTop
  let physicalHeight = scrollHeight
  const writes: number[] = []
  const container: Record<string, unknown> = {
    writes,
    moveTo(nextPhysicalTop: number) {
      physicalTop = Math.min(Math.max(nextPhysicalTop, 0), physicalHeight - clientHeight)
    },
    setScrollHeight(nextScrollHeight: number) {
      physicalHeight = nextScrollHeight
      physicalTop = Math.min(physicalTop, Math.max(physicalHeight - clientHeight, 0))
    }
  }

  if (supportsScrollEnd) container.onscrollend = null

  Object.defineProperties(container, {
    clientHeight: { get: () => clientHeight },
    scrollHeight: { get: () => physicalHeight },
    scrollTop: {
      get: () => physicalTop,
      set: (nextPhysicalTop: number) => {
        writes.push(nextPhysicalTop)
        physicalTop = Math.min(
          Math.max(nextPhysicalTop, 0),
          Math.max(physicalHeight - clientHeight, 0)
        )
      }
    }
  })

  return container as unknown as TestContainer
}

async function flushControllerQueue(): Promise<void> {
  for (let index = 0; index < 5; index += 1) {
    await Promise.resolve()
    await nextTick()
  }
}

async function consumeProgrammaticScroll(
  controller: HybridScrollController
): Promise<void> {
  controller.onScroll()
  await controller.onScrollEnd()
}

function createController({
  logicalTop,
  totalHeight,
  viewportHeight = 100,
  container = createContainer({ clientHeight: viewportHeight })
}: {
  logicalTop: number
  totalHeight: number
  viewportHeight?: number
  container?: TestContainer
}) {
  const prefetchStore = usePrefetchStore('tempId')
  const scrollTopStore = useScrollTopStore('tempId')
  prefetchStore.totalHeight = totalHeight
  scrollTopStore.scrollTop = logicalTop
  const controller = handleScroll(
    shallowRef<HTMLElement | null>(container),
    shallowRef(viewportHeight),
    'tempId',
    { compensationAnchorPx: 100 }
  )

  return { container, controller, prefetchStore, scrollTopStore }
}

describe('selectHybridScrollMode', () => {
  it('uses inclusive 3,000 px native edge zones', () => {
    expect(selectHybridScrollMode(3_000, 10_000, 'compensated')).toBe('native-top')
    expect(selectHybridScrollMode(3_001, 10_000, 'native-top')).toBe('compensated')
    expect(selectHybridScrollMode(6_999, 10_000, 'native-bottom')).toBe('compensated')
    expect(selectHybridScrollMode(7_000, 10_000, 'compensated')).toBe('native-bottom')
  })

  it('keeps short documents entirely native and switches at their midpoint', () => {
    expect(selectHybridScrollMode(2_447, 4_896, 'native-bottom')).toBe('native-top')
    expect(selectHybridScrollMode(2_448, 4_896, 'native-top')).toBe('native-top')
    expect(selectHybridScrollMode(2_448, 4_896, 'native-bottom')).toBe('native-top')
    expect(selectHybridScrollMode(2_449, 4_896, 'native-top')).toBe('native-bottom')
    expect(selectHybridScrollMode(0, 0, 'native-bottom')).toBe('native-top')
  })
})

describe('hybrid virtual scroll controller', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    vi.useFakeTimers()
  })

  it.each([
    {
      name: 'top',
      logicalTop: 2_000,
      expectedMode: 'native-top',
      expectedOrigin: 0,
      expectedPhysicalTop: 2_000
    },
    {
      name: 'middle',
      logicalTop: 5_000,
      expectedMode: 'compensated',
      expectedOrigin: -4_900,
      expectedPhysicalTop: 100
    },
    {
      name: 'bottom',
      logicalTop: 8_896,
      expectedMode: 'native-bottom',
      expectedOrigin: 590_004,
      expectedPhysicalTop: 598_900
    }
  ] as const)(
    'synchronizes a logical jump into the $name projection',
    async ({ logicalTop, expectedMode, expectedOrigin, expectedPhysicalTop }) => {
      const { container, controller } = createController({
        logicalTop,
        totalHeight: 10_000
      })

      await controller.syncToLogicalPosition('external')

      expect(controller.mode.value).toBe(expectedMode)
      expect(controller.projectionOrigin.value).toBe(expectedOrigin)
      expect(container.scrollTop).toBe(expectedPhysicalTop)
      expect(container.scrollTop - controller.projectionOrigin.value).toBe(logicalTop)
      expect(controller.minimumBufferHeight.value).toBe(600_000)
      controller.cancel()
    }
  )

  it('switches from compensated to native-top during the same gesture', async () => {
    const { container, controller, scrollTopStore } = createController({
      logicalTop: 3_050,
      totalHeight: 10_000
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    container.moveTo(40)
    controller.onScroll()
    await flushControllerQueue()

    expect(controller.mode.value).toBe('native-top')
    expect(controller.projectionOrigin.value).toBe(0)
    expect(controller.effectiveScrollTop.value).toBe(2_990)
    expect(scrollTopStore.scrollTop).toBe(2_990)
    expect(container.scrollTop).toBe(2_990)
    expect(container.writes).toEqual([2_990])

    // Chrome emits an internal scroll + scrollend for the rebase before inertia resumes.
    controller.onScroll()
    await controller.onScrollEnd()
    expect(container.writes).toEqual([2_990])

    container.moveTo(2_940)
    controller.onScroll()
    await controller.onScrollEnd()
    expect(scrollTopStore.scrollTop).toBe(2_940)
    expect(container.writes).toEqual([2_990])
    controller.cancel()
  })

  it('switches from native-top back to the compensated anchor immediately', async () => {
    const { container, controller } = createController({
      logicalTop: 2_900,
      totalHeight: 10_000
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    container.moveTo(3_100)
    controller.onScroll()
    await flushControllerQueue()

    expect(controller.mode.value).toBe('compensated')
    expect(controller.projectionOrigin.value).toBe(-3_000)
    expect(controller.effectiveScrollTop.value).toBe(3_100)
    expect(container.scrollTop).toBe(100)
    expect(container.writes).toEqual([100])
    controller.cancel()
  })

  it('hands off to native-bottom without assuming physical and logical heights match', async () => {
    const logicalUpperBound = 9_896
    const { container, controller } = createController({
      logicalTop: logicalUpperBound - 3_050,
      totalHeight: 10_000
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    container.moveTo(200)
    controller.onScroll()
    await flushControllerQueue()

    expect(controller.mode.value).toBe('native-bottom')
    expect(controller.projectionOrigin.value).toBe(590_004)
    expect(controller.effectiveScrollTop.value).toBe(logicalUpperBound - 2_950)
    expect(container.scrollTop).toBe(596_950)
    expect(container.scrollTop - controller.projectionOrigin.value).toBe(
      logicalUpperBound - 2_950
    )
    expect(container.writes).toEqual([596_950])
    controller.cancel()
  })

  it('re-anchors only compensated scrollend transactions', async () => {
    const { container, controller, scrollTopStore } = createController({
      logicalTop: 5_000,
      totalHeight: 10_000
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    container.moveTo(125)
    controller.onScroll()
    await controller.onScrollEnd()

    expect(scrollTopStore.scrollTop).toBe(5_025)
    expect(controller.projectionOrigin.value).toBe(-4_925)
    expect(container.scrollTop).toBe(100)
    expect(container.writes).toEqual([100])
    controller.cancel()
  })

  it('never writes scrollTop for repeated outward input at a native boundary', async () => {
    const { container, controller } = createController({ logicalTop: 0, totalHeight: 10_000 })
    await controller.syncToLogicalPosition('external')
    container.writes.length = 0

    controller.onScroll()
    controller.onScroll()
    await controller.onScrollEnd()

    expect(controller.mode.value).toBe('native-top')
    expect(container.scrollTop).toBe(0)
    expect(container.writes).toEqual([])
    expect(controller.getDebugSnapshot().internalWriteCount).toBe(0)
    controller.cancel()
  })

  it('projects a 120 million px document from its physical bottom cap', async () => {
    const viewportHeight = 675
    const totalHeight = 120_000_000
    const logicalUpperBound = 119_999_321
    const container = createContainer({
      scrollHeight: 33_554_428,
      clientHeight: viewportHeight
    })
    const { controller } = createController({
      logicalTop: logicalUpperBound - 1_000,
      totalHeight,
      viewportHeight,
      container
    })

    await controller.syncToLogicalPosition('external')

    const physicalMaximum = 33_553_753
    expect(controller.mode.value).toBe('native-bottom')
    expect(controller.projectionOrigin.value).toBe(physicalMaximum - logicalUpperBound)
    expect(container.scrollTop).toBe(physicalMaximum - 1_000)
    expect(container.scrollTop - controller.projectionOrigin.value).toBe(
      logicalUpperBound - 1_000
    )
    controller.cancel()
  })

  it('keeps the capped physical bottom fixed while logical geometry moves', async () => {
    const viewportHeight = 675
    const totalHeight = 120_000_000
    const container = createContainer({
      scrollHeight: 33_554_428,
      clientHeight: viewportHeight
    })
    const { controller, prefetchStore, scrollTopStore } = createController({
      logicalTop: 119_999_321,
      totalHeight,
      viewportHeight,
      container
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    prefetchStore.totalHeight += 500
    await controller.reconcileGeometry(500)

    expect(scrollTopStore.scrollTop).toBe(119_999_821)
    expect(container.scrollTop).toBe(33_553_753)
    expect(controller.projectionOrigin.value).toBe(33_553_753 - 119_999_821)
    expect(container.writes).toEqual([])
    controller.cancel()
  })

  it('captures the old bottom position before a height shrink clamps the DOM', async () => {
    const totalHeight = 1_000_000
    const container = createContainer({ scrollHeight: totalHeight })
    const { controller, prefetchStore, scrollTopStore } = createController({
      logicalTop: 999_896,
      totalHeight,
      container
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0

    prefetchStore.totalHeight -= 100
    const reconciliation = controller.reconcileGeometry(-100)
    container.setScrollHeight(999_900)
    await reconciliation

    expect(controller.logicalUpperBound.value).toBe(999_796)
    expect(scrollTopStore.scrollTop).toBe(999_796)
    expect(container.scrollTop).toBe(999_800)
    expect(controller.projectionOrigin.value).toBe(4)
    expect(container.writes).toEqual([])
    controller.cancel()
  })

  it('coalesces multiple row shifts from one Vue flush', async () => {
    const { container, controller, scrollTopStore } = createController({
      logicalTop: 5_000,
      totalHeight: 10_000
    })
    await controller.syncToLogicalPosition('external')
    await consumeProgrammaticScroll(controller)
    container.writes.length = 0
    const generationBefore = controller.generation.value

    const first = controller.reconcileGeometry(10)
    const second = controller.reconcileGeometry(15)
    await Promise.all([first, second])

    expect(scrollTopStore.scrollTop).toBe(5_025)
    expect(controller.generation.value).toBe(generationBefore + 1)
    expect(container.scrollTop).toBe(100)
    expect(container.writes).toEqual([])
    controller.cancel()
  })

  it('uses native scrolling and real content height for short documents', async () => {
    const container = createContainer({ scrollHeight: 5_000 })
    const { controller, scrollTopStore } = createController({
      logicalTop: 3_000,
      totalHeight: 5_000,
      container
    })

    await controller.syncToLogicalPosition('external')

    expect(controller.minimumBufferHeight.value).toBe(0)
    expect(controller.mode.value).toBe('native-bottom')
    expect(controller.projectionOrigin.value).toBe(4)
    expect(container.scrollTop).toBe(3_004)
    expect(scrollTopStore.scrollTop).toBe(3_000)
    controller.cancel()
  })

  it('settles with the 200 ms fallback when scrollend is unavailable', async () => {
    const container = createContainer({ supportsScrollEnd: false })
    const { controller, scrollTopStore } = createController({
      logicalTop: 1_000,
      totalHeight: 10_000,
      container
    })
    await controller.syncToLogicalPosition('external')
    controller.onScroll()
    container.moveTo(1_100)
    controller.onScroll()

    await vi.advanceTimersByTimeAsync(199)
    expect(scrollTopStore.scrollTop).toBe(1_000)
    await vi.advanceTimersByTimeAsync(1)
    await flushControllerQueue()

    expect(scrollTopStore.scrollTop).toBe(1_100)
    expect(container.scrollTop).toBe(1_100)
    controller.cancel()
  })
})
