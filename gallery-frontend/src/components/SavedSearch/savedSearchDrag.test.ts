import { describe, expect, it } from 'vitest'
import { getSavedSearchDragTiming } from './savedSearchDrag'

describe('getSavedSearchDragTiming', () => {
  it('starts dragging immediately on desktop regardless of viewport width', () => {
    expect(getSavedSearchDragTiming(false)).toEqual({
      delay: 0,
      delayOnTouchOnly: false
    })
  })

  it('uses a touch-only long press on mobile devices', () => {
    expect(getSavedSearchDragTiming(true)).toEqual({
      delay: 350,
      delayOnTouchOnly: true
    })
  })
})
