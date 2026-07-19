import { describe, expect, it } from 'vitest'
import { computeOffSetSumOfAboveRowsIndex } from '@utils/rowOffset'

describe('row fetch offset range calculation', () => {
  it('preserves the existing above-row offset semantics in one pass', () => {
    const rows = new Map([
      [0, { rowIndex: 0, topPixelAccumulated: 0, offset: 0 }],
      [1, { rowIndex: 1, topPixelAccumulated: 2400, offset: 12 }],
      [2, { rowIndex: 2, topPixelAccumulated: 4800, offset: 7 }]
    ])
    const offsets = new Map([
      [0, 12],
      [1, -5],
      [2, 9]
    ])

    expect(computeOffSetSumOfAboveRowsIndex(0, rows, offsets)).toBe(0)
    expect(computeOffSetSumOfAboveRowsIndex(3000, rows, offsets)).toBe(7)
    expect(computeOffSetSumOfAboveRowsIndex(10_000, rows, offsets)).toBe(16)
  })
})
