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

/** Projects a logical row position from the top of the physical buffer. */
export function projectVirtualTop(logicalTop: number, projectionOrigin: number): number {
  return logicalTop + projectionOrigin
}

/**
 * Returns the small distance between a logical row and the bottom of the physical buffer.
 *
 * Native-bottom mode can represent a logical document that is much taller than Chrome's
 * physical layout limit. Anchoring from the bottom keeps the CSS transform bounded even when
 * both the logical position and the physical scroll range are tens of millions of pixels.
 */
export function projectVirtualBottom(
  logicalTop: number,
  logicalUpperBound: number,
  viewportHeight: number
): number {
  return viewportHeight + logicalUpperBound - logicalTop
}

/** Returns a row's bounded position inside a visible-row group. */
export function projectRelativeTop(logicalTop: number, groupLogicalTop: number): number {
  return logicalTop - groupLogicalTop
}
