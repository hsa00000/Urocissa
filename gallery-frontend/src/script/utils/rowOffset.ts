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
