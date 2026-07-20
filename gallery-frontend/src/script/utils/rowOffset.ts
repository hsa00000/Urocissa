export function computeOffSetSumOfAboveRowsIndex(
  scrollTop: number,
  rowData: ReadonlyMap<number, { rowIndex: number; topPixelAccumulated: number; offset: number }>,
  offsets: ReadonlyMap<number, number>
) {
  let offsetSum = 0

  for (const row of rowData.values()) {
    if (row.topPixelAccumulated + row.offset < scrollTop) {
      const offset = offsets.get(row.rowIndex)
      if (offset !== undefined) {
        offsetSum += offset
      } else {
        console.error('offset is undefined')
      }
    }
  }

  return offsetSum
}

/**
 * Projects a logical row position into the bounded physical buffer coordinate space.
 *
 * Keep the subtraction in JavaScript instead of splitting it across parent and child CSS
 * transforms. At million-photo scale both logical values can exceed browser rendering limits
 * even though their viewport-relative result remains small.
 */
export function projectVirtualTop(
  logicalTop: number,
  committedScrollTop: number,
  bufferHeight: number
): number {
  return logicalTop - committedScrollTop + bufferHeight / 3
}

/** Returns a row's bounded position inside a visible-row group. */
export function projectRelativeTop(logicalTop: number, groupLogicalTop: number): number {
  return logicalTop - groupLogicalTop
}
