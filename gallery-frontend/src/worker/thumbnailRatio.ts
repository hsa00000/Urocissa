import type { Row } from '@type/types'

export function clampRowDisplayRatios(row: Row): Row {
  return {
    ...row,
    displayElements: row.displayElements.map((displayElement) => {
      const { displayWidth, displayHeight } = displayElement
      if (displayWidth <= 0 || displayHeight <= 0) {
        return displayElement
      }

      const ratio = displayWidth / displayHeight
      if (ratio > 2) {
        return { ...displayElement, displayWidth: displayHeight * 2 }
      }
      if (ratio < 0.5) {
        return { ...displayElement, displayHeight: displayWidth * 2 }
      }
      return displayElement
    })
  }
}
