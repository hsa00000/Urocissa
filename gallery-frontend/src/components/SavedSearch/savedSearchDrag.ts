const MOBILE_TOUCH_DRAG_DELAY_MS = 350

export interface SavedSearchDragTiming {
  delay: number
  delayOnTouchOnly: boolean
}

export function getSavedSearchDragTiming(isMobile: boolean): SavedSearchDragTiming {
  return {
    delay: isMobile ? MOBILE_TOUCH_DRAG_DELAY_MS : 0,
    delayOnTouchOnly: isMobile
  }
}
