import { IsolationId, Row } from '@/script/common/types'
import { fetchRowInWorker } from '@/script/inWorker/fetchRowInWorker'
import { usePrefetchStore } from '@/store/prefetchStore'
import { useLocationStore } from '@/store/locationStore'
import { useRowStore } from '@/store/rowStore'
import { Ref, ref, toRaw, watch } from 'vue'
import { useScrollTopStore } from '@/store/scrollTopStore'
import { getArrayValue, getMapValue, getScrollUpperBound } from '@utils/getter'

/**
 * Finds and returns rows that overlap within the given range.
 * The resulting rows are sorted by their top position (`topPixelAccumulated`) plus offset.
 *
 * @param rowMap - A map containing all rows.
 * @param startRange - The starting pixel position of the vertical range.
 * @param endRange - The ending pixel position of the vertical range.
 * @returns A sorted array of rows that fall within the specified vertical range.
 */
function findRowInRange(rowMap: Map<number, Row>, startRange: number, endRange: number): Row[] {
  const visibleRows = []

  for (const [, row] of rowMap) {
    if (
      row.topPixelAccumulated + row.rowHeight + row.offset >= startRange &&
      row.topPixelAccumulated + row.offset < endRange
    ) {
      visibleRows.push(row)
    }
  }

  // Sort the rows by their top position (topPixelAccumulated) plus offset
  visibleRows.sort((a, b) => a.topPixelAccumulated + a.offset - (b.topPixelAccumulated + b.offset))

  return visibleRows
}

/**
 * Computes the rows currently visible in the viewport, taking into account rows
 * from the previous frame to ensure smooth transitions.
 *
 * @param lastVisibleRow - A map of rows visible in the last frame.
 * @param rowData -  A map containing all rows.
 * @param startHeight - The starting height of the current viewport.
 * @param endHeight - The ending height of the current viewport.
 * @returns An array of rows currently visible in the viewport.
 */
function getCurrentVisibleRows(
  lastVisibleRow: Map<number, Row>,
  startHeight: number,
  endHeight: number,
  isolationId: IsolationId
): Row[] {
  const rowStore = useRowStore(isolationId)
  let extraShift = 0

  // Find rows within the current viewport range that were visible in the previous frame
  const rowsInRange = findRowInRange(lastVisibleRow, startHeight, endHeight)

  if (rowsInRange.length > 0) {
    // If there are rows from the previous frame within the current viewport,
    // calculate the shift in row offsets.
    const lastRow = getArrayValue(rowsInRange, rowsInRange.length - 1)
    const lastRowOffset = lastRow.offset
    const currentRowOffset = getMapValue(rowStore.rowData, lastRow.rowIndex).offset
    extraShift = currentRowOffset - lastRowOffset
  }

  if (rowsInRange.length > 0 && extraShift === 0) {
    // If there are rows from the previous frame visible in the current viewport
    // and there is no offset shift, return these rows directly as they are
    // correctly positioned.
    return rowsInRange
  } else {
    // If no rows from the previous frame are visible or there is an offset shift:
    // - If there are no visible rows from the previous frame, return the rows
    //   within the new viewport range directly.
    // - If there is an offset shift (i.e., some rows from the previous frame might
    //   still be visible), adjust the viewport range by the offset shift. This ensures
    //   that rows which were still within the viewport from the previous frame are
    //   included. The offset shift accounts for significant changes in row positions
    //   between frames, which might cause the previous `findRowInRange` function to
    //   miss these rows if the offsets changed drastically.

    const currentRowsInRange = findRowInRange(
      rowStore.rowData,
      startHeight + extraShift,
      endHeight + extraShift
    )

    return currentRowsInRange
  }
}

/**
 * Updates `visibleRows` by adding a row to the end and one to the beginning.
 *
 * @param visibleRows - Array of currently visible rows.
 * @param rowData - Map of all rows by index.
 */
function appendAndPrependRow(visibleRows: Ref<Row[]>, isolationId: IsolationId) {
  // assume visibleRows.value.length > 0
  const rowStore = useRowStore(isolationId)

  const lastRowIndex = getArrayValue(visibleRows.value, visibleRows.value.length - 1).rowIndex
  const appendRow = rowStore.rowData.get(lastRowIndex + 1)

  if (appendRow) {
    visibleRows.value.push(appendRow)
  }

  const firstRowIndex = getArrayValue(visibleRows.value, 0).rowIndex
  const prependRow = rowStore.rowData.get(firstRowIndex - 1)

  if (prependRow) {
    visibleRows.value.unshift(prependRow)
  }
}

/**
 * Filters `visibleRows` to keep only the row at the location specified by `locationStore.anchor`.
 * Clears the anchor in `locationStore` if a row is found.
 *
 * @param visibleRows - Array of currently visible rows.
 */
function filterRowForLocation(visibleRows: Ref<Row[]>, isolationId: IsolationId) {
  const locationStore = useLocationStore(isolationId)
  if (locationStore.anchor !== null) {
    visibleRows.value = visibleRows.value.filter((rowData) => {
      return rowData.rowIndex === locationStore.anchor
    })

    if (visibleRows.value.length > 0) {
      locationStore.anchor = null
    }
  }
}

/**
 * Adjusts the `scrollTop` value to ensure the visible rows are properly aligned.
 * Calculates the necessary offset shift based on the difference between the current and last known offsets of the last visible row.
 *
 * @param visibleRows - Array of currently visible rows.
 * @param scrollTop - Current scroll position.
 * @param scrollingBound - Maximum allowed scroll position.
 */
function scrollTopOffsetFix(
  visibleRows: Ref<Row[]>,
  scrollingBound: number,
  isolationId: IsolationId
) {
  const rowStore = useRowStore(isolationId)
  const scrollTopStore = useScrollTopStore(isolationId)

  const lastRow = visibleRows.value.findLast((row) => rowStore.lastVisibleRow.has(row.rowIndex))

  if (lastRow) {
    const lastKnownRow = rowStore.lastVisibleRow.get(lastRow.rowIndex)
    if (lastKnownRow) {
      // Compute the difference between the current and last known offset
      const shift = lastRow.offset - lastKnownRow.offset

      // Adjust scrollTop while ensuring it does not exceed the scrollingBound
      scrollTopStore.scrollTop = Math.min(scrollTopStore.scrollTop + shift, scrollingBound)
    }
  }
}

/**
 * Updates `rowStore.lastVisibleRow` with the current visible rows.
 *
 * @param visibleRows - Array of currently visible rows.
 */
function updateLastVisibleRow(visibleRows: Ref<Row[]>, isolationId: IsolationId) {
  const rowStore = useRowStore(isolationId)
  rowStore.lastVisibleRow.clear()
  visibleRows.value.forEach((row) => {
    rowStore.lastVisibleRow.set(row.rowIndex, structuredClone(toRaw(row)))
  })
}

/**
 * Updates `locationStore.locationIndex` based on the current `scrollTop` position.
 *
 * @param visibleRows - Array of currently visible rows.
 * @param scrollTop - Current scroll position.
 */
function updateLocationIndex(visibleRows: Ref<Row[]>, scrollTop: number, isolationId: IsolationId) {
  const locationStore = useLocationStore(isolationId)
  for (const row of visibleRows.value) {
    if (row.topPixelAccumulated + row.rowHeight + row.offset >= scrollTop) {
      const topPixelAccumulatedOffseted = row.topPixelAccumulated + row.offset

      for (let index = 0; index < row.displayElements.length; index++) {
        const displayElement = getArrayValue(row.displayElements, index)
        if (
          topPixelAccumulatedOffseted +
            displayElement.displayTopPixelAccumulated +
            displayElement.displayHeight >=
          scrollTop
        ) {
          locationStore.locationIndex = row.start + index
          break
        }
      }
      break
    }
  }
}

/**
 * Updates `lastRowBottom` with the bottom position of the last visible row.
 *
 * @param visibleRows - Array of currently visible rows.
 * @param lastRowBottom - Reference to store the bottom position of the last row.
 */
function updateLastRowBottom(
  visibleRows: Ref<Row[]>,
  lastRowBottom: Ref<number>,
  endHeight: number,
  isolationId: IsolationId
) {
  const prefetchStore = usePrefetchStore(isolationId)
  const lastRow = visibleRows.value[visibleRows.value.length - 1]
  if (lastRow) {
    const lastRowBottomComputed = lastRow.topPixelAccumulated + lastRow.offset + lastRow.rowHeight
    lastRowBottom.value = lastRowBottomComputed
    if (lastRowBottomComputed <= endHeight && lastRow.end < prefetchStore.dataLength) {
      const lastRowIndex = lastRow.rowIndex
      fetchRowInWorker(lastRowIndex + 1, isolationId)
      setTimeout(() => {
        prefetchStore.updateVisibleRowTrigger = !prefetchStore.updateVisibleRowTrigger
      }, 500)
    }
  }
}

/**
 * Manages and updates the visible rows based on the current scroll position and viewport dimensions.
 * It handles the addition and removal of rows, adjusts the scroll position if needed, and updates related stores.
 *
 * @param imageContainerRef - Reference to the container element that holds the rows.
 * @param scrollTop - Current vertical scroll position.
 * @param startHeight - Starting pixel position of the viewport.
 * @param endHeight - Ending pixel position of the viewport.
 * @param lastRowBottom - Reference to store the bottom position of the last visible row.
 * @param windowHeight - Height of the visible window area.
 * @returns An object containing the `updateVisibleRows` function.
 */
export function useUpdateVisibleRows(
  imageContainerRef: Ref<HTMLElement | null>,
  startHeight: Ref<number>,
  endHeight: Ref<number>,
  lastRowBottom: Ref<number>,
  windowHeight: Ref<number>,
  isolationId: IsolationId
) {
  const visibleRows: Ref<Row[]> = ref<Row[]>([])
  const prefetchStore = usePrefetchStore(isolationId)
  const rowStore = useRowStore(isolationId)
  const scrollTopStore = useScrollTopStore(isolationId)

  const updateVisibleRows = () => {
    if (imageContainerRef.value) {
      visibleRows.value = getCurrentVisibleRows(
        rowStore.lastVisibleRow,
        startHeight.value,
        endHeight.value,
        isolationId
      )

      if (visibleRows.value.length > 0) {
        // The logic in getCurrentVisibleRows might miss the top and bottom rows, so we add them back
        appendAndPrependRow(visibleRows, isolationId)

        filterRowForLocation(visibleRows, isolationId)

        scrollTopOffsetFix(
          visibleRows,
          Math.max(getScrollUpperBound(prefetchStore.totalHeight, windowHeight.value), 0),
          isolationId
        )
      }
      updateLastVisibleRow(visibleRows, isolationId)
      updateLocationIndex(visibleRows, scrollTopStore.scrollTop, isolationId)
      updateLastRowBottom(visibleRows, lastRowBottom, endHeight.value, isolationId)
    }
  }

  // Watch dependencies and trigger updateVisibleRows when any change occurs
  watch(
    [
      imageContainerRef,
      () => scrollTopStore.scrollTop,
      () => prefetchStore.updateVisibleRowTrigger,
      () => document.visibilityState
    ],
    updateVisibleRows,
    { immediate: true }
  )

  return { visibleRows }
}
