import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { shallowRef } from 'vue'
import type { Row } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useRowStore } from '@/store/rowStore'
import { useScrollTopStore } from '@/store/scrollTopStore'

const fetchRowInWorkerMock = vi.hoisted(() => vi.fn<() => Promise<void>>())

vi.mock('@/api/fetchRow', () => ({
  fetchRowInWorker: fetchRowInWorkerMock
}))

import {
  getCurrentVisibleRows,
  scrollTopOffsetFix,
  updateLastRowBottom,
  updateLastVisibleRow
} from './useUpdateVisibleRows'

function createRow(rowIndex = 0, offset = 0): Row {
  return {
    start: rowIndex * 20,
    end: rowIndex * 20 + 19,
    rowHeight: 2400,
    displayElements: [
      {
        displayWidth: 100,
        displayHeight: 75,
        displayTopPixelAccumulated: 0
      }
    ],
    topPixelAccumulated: rowIndex * 2400,
    rowIndex,
    offset
  }
}

describe('visible-row geometry snapshots', () => {
  beforeEach(() => {
    setActivePinia(createPinia())
    fetchRowInWorkerMock.mockReset().mockResolvedValue(undefined)
  })

  it('reuses unchanged geometry and resolves current rows instead of rendering snapshots', () => {
    const rowStore = useRowStore('tempId')
    rowStore.rowData.set(0, createRow())
    const currentRow = rowStore.rowData.get(0)
    if (currentRow === undefined) throw new Error('row fixture was not stored')
    const visibleRows = shallowRef([currentRow])

    updateLastVisibleRow(visibleRows, rowStore)
    const firstSnapshotMap = rowStore.lastVisibleRow
    const firstSnapshot = firstSnapshotMap.get(0)
    expect(firstSnapshot).toEqual({
      rowIndex: 0,
      topPixelAccumulated: 0,
      rowHeight: 2400,
      offset: 0
    })
    expect(firstSnapshot).not.toHaveProperty('displayElements')

    updateLastVisibleRow(visibleRows, rowStore)
    expect(rowStore.lastVisibleRow).toBe(firstSnapshotMap)

    const resolvedRows = getCurrentVisibleRows(rowStore.lastVisibleRow, 0, 1000, rowStore)
    expect(resolvedRows).toHaveLength(1)
    expect(resolvedRows[0]).toBe(currentRow)
  })

  it('applies an offset shift once and then snapshots the new geometry', () => {
    const rowStore = useRowStore('tempId')
    const scrollTopStore = useScrollTopStore('tempId')
    rowStore.rowData.set(0, createRow())
    const currentRow = rowStore.rowData.get(0)
    if (currentRow === undefined) throw new Error('row fixture was not stored')
    const visibleRows = shallowRef([currentRow])
    scrollTopStore.scrollTop = 100

    updateLastVisibleRow(visibleRows, rowStore)
    currentRow.offset = 25
    scrollTopOffsetFix(visibleRows, 1000, rowStore, scrollTopStore)
    expect(scrollTopStore.scrollTop).toBe(125)

    updateLastVisibleRow(visibleRows, rowStore)
    scrollTopOffsetFix(visibleRows, 1000, rowStore, scrollTopStore)
    expect(scrollTopStore.scrollTop).toBe(125)
  })

  it('requests a missing row without scheduling a self-trigger timer', () => {
    const prefetchStore = usePrefetchStore('tempId')
    prefetchStore.dataLength = 100
    const visibleRows = shallowRef([createRow()])
    const lastRowBottom = shallowRef(0)
    const setTimeoutSpy = vi.spyOn(globalThis, 'setTimeout')

    updateLastRowBottom(visibleRows, lastRowBottom, 2400, prefetchStore, 'tempId')

    expect(fetchRowInWorkerMock).toHaveBeenCalledTimes(1)
    expect(fetchRowInWorkerMock).toHaveBeenCalledWith(1, 'tempId')
    expect(prefetchStore.updateVisibleRowTrigger).toBe(false)
    expect(setTimeoutSpy).not.toHaveBeenCalled()

    setTimeoutSpy.mockRestore()
  })
})
