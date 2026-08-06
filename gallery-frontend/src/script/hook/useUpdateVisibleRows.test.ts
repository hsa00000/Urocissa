import { beforeEach, describe, expect, it, vi } from 'vitest'
import { createPinia, setActivePinia } from 'pinia'
import { shallowRef } from 'vue'
import type { Row } from '@type/types'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useRowStore } from '@/store/rowStore'

const fetchRowInWorkerMock = vi.hoisted(() => vi.fn<() => Promise<void>>())

vi.mock('@/api/fetchRow', () => ({
  fetchRowInWorker: fetchRowInWorkerMock
}))

import {
  getCurrentVisibleRows,
  getVisibleRowAnchorShift,
  publishVisibleRowsIfChanged,
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

  it('reports a complete logical anchor shift once and then snapshots it', () => {
    const rowStore = useRowStore('tempId')
    rowStore.rowData.set(0, createRow())
    const currentRow = rowStore.rowData.get(0)
    if (currentRow === undefined) throw new Error('row fixture was not stored')
    const visibleRows = shallowRef([currentRow])

    updateLastVisibleRow(visibleRows, rowStore)
    currentRow.offset = 25
    currentRow.topPixelAccumulated = 100
    expect(getVisibleRowAnchorShift(visibleRows, rowStore)).toBe(125)

    updateLastVisibleRow(visibleRows, rowStore)
    expect(getVisibleRowAnchorShift(visibleRows, rowStore)).toBe(0)
  })

  it('uses the complete accumulated-top shift when recovering visible rows', () => {
    const rowStore = useRowStore('tempId')
    const currentRow = createRow()
    rowStore.rowData.set(0, currentRow)
    const visibleRows = shallowRef([currentRow])
    updateLastVisibleRow(visibleRows, rowStore)

    currentRow.topPixelAccumulated = 2_000
    const resolvedRows = getCurrentVisibleRows(rowStore.lastVisibleRow, 0, 1_000, rowStore)

    expect(resolvedRows).toEqual([currentRow])
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

  it('reuses the visible-row array when row identity and order are unchanged', () => {
    const firstRow = createRow(0)
    const secondRow = createRow(1)
    const visibleRows = shallowRef([firstRow, secondRow])
    const originalRows = visibleRows.value

    expect(publishVisibleRowsIfChanged(visibleRows, [firstRow, secondRow])).toBe(false)
    expect(visibleRows.value).toBe(originalRows)

    expect(publishVisibleRowsIfChanged(visibleRows, [secondRow, firstRow])).toBe(true)
    expect(visibleRows.value).toEqual([secondRow, firstRow])
    expect(visibleRows.value).not.toBe(originalRows)
  })
})
