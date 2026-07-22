export type PriorityEscapeEvent = Pick<
  KeyboardEvent,
  'key' | 'preventDefault' | 'stopImmediatePropagation' | 'stopPropagation'
>

/**
 * Consumes Escape before lower-priority keyboard handlers and runs the supplied close action.
 */
export function handlePriorityEscape(
  event: PriorityEscapeEvent,
  onEscape: () => void
): boolean {
  if (event.key !== 'Escape') return false

  event.preventDefault()
  event.stopPropagation()
  event.stopImmediatePropagation()
  onEscape()
  return true
}
