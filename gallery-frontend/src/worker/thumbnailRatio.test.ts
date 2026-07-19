import type { Row } from '@type/types'
import { describe, expect, it } from 'vitest'
import { clampRowDisplayRatios } from './thumbnailRatio'

function createRow(dimensions: [number, number][]): Row {
  return {
    start: 0,
    end: dimensions.length - 1,
    rowHeight: 2400,
    displayElements: dimensions.map(([displayWidth, displayHeight]) => ({
      displayWidth,
      displayHeight,
      displayTopPixelAccumulated: 0
    })),
    topPixelAccumulated: 0,
    rowIndex: 0,
    offset: 0
  }
}

describe('clampRowDisplayRatios', () => {
  it('limits wide and tall thumbnail containers to the 1:2 through 2:1 range', () => {
    const row = createRow([
      [400, 100],
      [100, 400]
    ])

    const result = clampRowDisplayRatios(row)

    expect(result.displayElements).toMatchObject([
      { displayWidth: 200, displayHeight: 100 },
      { displayWidth: 100, displayHeight: 200 }
    ])
  })

  it('leaves boundary ratios and invalid dimensions unchanged', () => {
    const row = createRow([
      [200, 100],
      [100, 200],
      [0, 100]
    ])

    expect(clampRowDisplayRatios(row).displayElements).toEqual(row.displayElements)
  })

  it('does not mutate the source row', () => {
    const row = createRow([[400, 100]])

    clampRowDisplayRatios(row)

    expect(row.displayElements[0]).toMatchObject({ displayWidth: 400, displayHeight: 100 })
  })
})
